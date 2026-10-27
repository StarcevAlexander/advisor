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
	messages := append(historyMessages(history, noTurnBlocks), anthropic.NewUserMessage(anthropic.NewTextBlock(request)))
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
	plan, found := lastPlanObject(raw)
	if !found {
		return Plan{}, ErrBadPlan
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

func lastPlanObject(raw string) (Plan, bool) {
	var plan Plan
	found := false
	for offset := 0; offset < len(raw); {
		start := strings.IndexByte(raw[offset:], '{')
		if start < 0 {
			break
		}
		start += offset
		decoder := json.NewDecoder(strings.NewReader(raw[start:]))
		var candidate Plan
		if err := decoder.Decode(&candidate); err == nil && strings.TrimSpace(candidate.Summary) != "" {
			plan, found = candidate, true
			offset = start + int(decoder.InputOffset())
			continue
		}
		offset = start + 1
	}
	return plan, found
}

func truncateRunes(text string, limit int) string {
	if utf8.RuneCountInString(text) <= limit {
		return text
	}
	return string([]rune(text)[:limit-1]) + "…"
}
