use crate::zai::{ChatMessage, CompletionRequest};

use super::constants::{
    MAX_PLAN_STEP_CHARS, MAX_PLAN_STEPS, MAX_PLAN_SUMMARY_CHARS, PLAN_MAX_OUTPUT_TOKENS,
    PLAN_REASONING_EFFORT, PLAN_SYSTEM_PROMPT, REFUSAL_REPLY,
};
use super::messages::{build_messages, history_messages};
use super::models::{ChatError, Plan, Planner, Turn};
use super::responder::Zai;

#[async_trait::async_trait]
impl Planner for Zai {
    async fn plan(&self, request: &str, history: &[Turn]) -> Result<Plan, ChatError> {
        let mut messages = history_messages(history, None);
        messages = build_messages(messages, request, Vec::new());
        messages.insert(0, ChatMessage::system(PLAN_SYSTEM_PROMPT));

        let completion = CompletionRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(PLAN_MAX_OUTPUT_TOKENS),
            reasoning_effort: Some(PLAN_REASONING_EFFORT.to_string()),
        };

        match self.client.complete(&completion).await {
            Ok(text) => parse_plan(&text),
            Err(crate::zai::Error::Refused) => Ok(Plan {
                summary: REFUSAL_REPLY.to_string(),
                steps: Vec::new(),
            }),
            Err(err) => Err(ChatError::Zai(err)),
        }
    }
}

pub fn parse_plan(raw: &str) -> Result<Plan, ChatError> {
    let Some(mut plan) = last_plan_object(raw) else {
        return Err(ChatError::BadPlan);
    };

    plan.summary = truncate_chars(plan.summary.trim(), MAX_PLAN_SUMMARY_CHARS);
    if plan.summary.is_empty() {
        return Err(ChatError::BadPlan);
    }

    let mut steps = Vec::with_capacity(MAX_PLAN_STEPS);
    for step in plan.steps {
        let step = step.trim();
        if step.is_empty() {
            continue;
        }
        steps.push(truncate_chars(step, MAX_PLAN_STEP_CHARS));
        if steps.len() == MAX_PLAN_STEPS {
            break;
        }
    }
    plan.steps = steps;
    Ok(plan)
}

fn last_plan_object(raw: &str) -> Option<Plan> {
    let mut plan: Option<Plan> = None;
    let mut offset = 0usize;
    let bytes = raw.as_bytes();
    while offset < bytes.len() {
        let Some(relative) = bytes[offset..].iter().position(|&b| b == b'{') else {
            break;
        };
        let start = offset + relative;
        let mut stream = serde_json::Deserializer::from_str(&raw[start..]).into_iter::<Plan>();
        match stream.next() {
            Some(Ok(candidate)) if !candidate.summary.trim().is_empty() => {
                plan = Some(candidate);
                offset = start + stream.byte_offset();
            }
            _ => {
                offset = start + 1;
            }
        }
    }
    plan
}

pub fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(limit.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_happy_path() {
        let raw = r#"{"summary":"поменяю кнопку","steps":["увеличу кнопку","поменяю цвет"]}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.summary, "поменяю кнопку");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn parse_plan_picks_last_valid_object() {
        let raw =
            r#"мусор {"summary":"первый","steps":["а"]} ещё {"summary":"второй","steps":["б"]}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.summary, "второй");
    }

    #[test]
    fn parse_plan_filters_empty_steps_and_caps_count() {
        let raw = r#"{"summary":"s","steps":["", " один ", "два", "три", "четыре", "пять", "шесть", "семь"]}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.steps.len(), MAX_PLAN_STEPS);
        assert_eq!(plan.steps[0], "один");
    }

    #[test]
    fn parse_plan_truncates_long_text() {
        let long_summary = "а".repeat(500);
        let plan = parse_plan(&format!(r#"{{"summary":"{long_summary}","steps":[]}}"#)).unwrap();
        assert_eq!(plan.summary.chars().count(), MAX_PLAN_SUMMARY_CHARS);
        assert!(plan.summary.ends_with('…'));
    }

    #[test]
    fn parse_plan_rejects_garbage_and_empty_summary() {
        assert!(matches!(parse_plan("нет json"), Err(ChatError::BadPlan)));
        assert!(matches!(
            parse_plan(r#"{"summary":"  ","steps":[]}"#),
            Err(ChatError::BadPlan)
        ));
    }

    #[test]
    fn parse_plan_skips_object_without_summary() {
        let raw = r#"{"steps":["а"]} {"summary":"годный","steps":["б"]}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.summary, "годный");
    }

    #[test]
    fn truncate_chars_short_text_unchanged() {
        assert_eq!(truncate_chars("abc", 5), "abc");
    }
}
