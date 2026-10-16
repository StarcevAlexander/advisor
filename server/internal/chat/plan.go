package chat

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"unicode/utf8"

	"github.com/anthropics/anthropic-sdk-go"
)

func (c *Claude) Plan(ctx context.Context, request string, history []Turn) (Plan, error) {
	messages := append(historyMessages(history), anthropic.NewUserMessage(anthropic.NewTextBlock(request)))
	response, err := c.client.Messages.New(ctx, anthropic.MessageNewParams{
		Model:        c.model,
		MaxTokens:    PlanMaxOutputTokens,
		System:       []anthropic.TextBlockParam{{Text: PlanSystemPrompt}},
		OutputConfig: anthropic.OutputConfigParam{Effort: anthropic.OutputConfigEffortMedium},
		Messages:     messages,
	})
	if err != nil {
		return Plan{}, fmt.Errorf("claude plan request: %w", err)
	}

	if response.StopReason == anthropic.StopReasonRefusal {
		return Plan{Summary: RefusalReply, Steps: []string{}}, nil
	}

	var text strings.Builder
	for _, block := range response.Content {
		if part, ok := block.AsAny().(anthropic.TextBlock); ok {
			text.WriteString(part.Text)
		}
	}
	return parsePlan(text.String())
}

func parsePlan(raw string) (Plan, error) {
	start := strings.Index(raw, "{")
	end := strings.LastIndex(raw, "}")
	if start < 0 || end <= start {
		return Plan{}, ErrBadPlan
	}

	var plan Plan
	if err := json.Unmarshal([]byte(raw[start:end+1]), &plan); err != nil {
		return Plan{}, fmt.Errorf("%w: %v", ErrBadPlan, err)
	}

	plan.Summary = truncateRunes(strings.TrimSpace(plan.Summary), MaxPlanSummaryRunes)
	if plan.Summary == "" {
		return Plan{}, ErrBadPlan
	}

	steps := make([]string, 0, MaxPlanSteps)
	for _, step := range plan.Steps {
		step = strings.TrimSpace(step)
		if step == "" {
			continue
		}
		steps = append(steps, truncateRunes(step, MaxPlanStepRunes))
		if len(steps) == MaxPlanSteps {
			break
		}
	}
	plan.Steps = steps
	return plan, nil
}

func truncateRunes(text string, limit int) string {
	if utf8.RuneCountInString(text) <= limit {
		return text
	}
	return string([]rune(text)[:limit-1]) + "…"
}
