package chat

import (
	"errors"
	"strings"
	"testing"
)

func TestParsePlan(t *testing.T) {
	tests := []struct {
		name      string
		raw       string
		wantSteps int
		wantErr   bool
	}{
		{name: "plain json", raw: `{"summary":"Сделаю","steps":["Один","Два"]}`, wantSteps: 2},
		{name: "fenced json", raw: "```json\n{\"summary\":\"Сделаю\",\"steps\":[\"Один\"]}\n```", wantSteps: 1},
		{name: "no steps is a question", raw: `{"summary":"Уточните, что именно","steps":[]}`, wantSteps: 0},
		{name: "blank steps dropped", raw: `{"summary":"Ок","steps":["  ","Один"]}`, wantSteps: 1},
		{name: "steps capped", raw: `{"summary":"Ок","steps":["1","2","3","4","5","6","7","8"]}`, wantSteps: MaxPlanSteps},
		{name: "no json", raw: "просто текст", wantErr: true},
		{name: "empty summary", raw: `{"summary":" ","steps":["Один"]}`, wantErr: true},
		{name: "broken json", raw: `{"summary":`, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			plan, err := parsePlan(tt.raw)
			if tt.wantErr {
				if !errors.Is(err, ErrBadPlan) {
					t.Fatalf("err = %v, want ErrBadPlan", err)
				}
				return
			}
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if len(plan.Steps) != tt.wantSteps {
				t.Errorf("steps = %d, want %d", len(plan.Steps), tt.wantSteps)
			}
			if plan.Steps == nil {
				t.Error("steps must be an empty list, not nil")
			}
		})
	}
}

func TestParsePlanTruncatesLongText(t *testing.T) {
	longStep := strings.Repeat("я", MaxPlanStepRunes+50)
	longSummary := strings.Repeat("я", MaxPlanSummaryRunes+50)

	plan, err := parsePlan(`{"summary":"` + longSummary + `","steps":["` + longStep + `"]}`)
	if err != nil {
		t.Fatal(err)
	}

	if got := len([]rune(plan.Steps[0])); got != MaxPlanStepRunes {
		t.Errorf("step runes = %d, want %d", got, MaxPlanStepRunes)
	}
	if got := len([]rune(plan.Summary)); got != MaxPlanSummaryRunes {
		t.Errorf("summary runes = %d, want %d", got, MaxPlanSummaryRunes)
	}
}
