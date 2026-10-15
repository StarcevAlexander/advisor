package httpapi

import (
	"context"
	"errors"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"advisor/server/internal/changes"
	"advisor/server/internal/chat"
)

type fakePlanner struct {
	plan     chat.Plan
	err      error
	requests []string
}

func (f *fakePlanner) Plan(_ context.Context, request string) (chat.Plan, error) {
	f.requests = append(f.requests, request)
	return f.plan, f.err
}

func postPlan(t *testing.T, planner chat.Planner, requester changes.Requester, password, body string) *httptest.ResponseRecorder {
	t.Helper()
	handler := NewHandler(Options{
		Responder:      &fakeResponder{},
		Planner:        planner,
		Changes:        requester,
		Password:       testPassword,
		AllowedOrigins: []string{testOrigin},
		Logger:         slog.New(slog.NewTextHandler(io.Discard, nil)),
	})
	request := httptest.NewRequest(http.MethodPost, "/api/change/plan", strings.NewReader(body))
	if password != "" {
		request.Header.Set("Authorization", BearerPrefix+password)
	}
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	return recorder
}

func TestChangePlan(t *testing.T) {
	planner := &fakePlanner{plan: chat.Plan{Summary: "Сделаю кнопку красной", Steps: []string{"Кнопка станет красной"}}}

	recorder := postPlan(t, planner, &fakeRequester{}, testPassword, `{"request":"  Сделай кнопку красной  "}`)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", recorder.Code, recorder.Body)
	}
	body := recorder.Body.String()
	for _, want := range []string{`"summary":"Сделаю кнопку красной"`, `"steps":["Кнопка станет красной"]`} {
		if !strings.Contains(body, want) {
			t.Errorf("body %s lacks %s", body, want)
		}
	}
	if len(planner.requests) != 1 || planner.requests[0] != "Сделай кнопку красной" {
		t.Errorf("requests = %v", planner.requests)
	}
}

func TestChangePlanRejections(t *testing.T) {
	long := strings.Repeat("я", MaxChangeRequestRunes+1)
	tests := []struct {
		name       string
		planner    chat.Planner
		requester  changes.Requester
		password   string
		body       string
		wantStatus int
	}{
		{name: "no password", planner: &fakePlanner{}, requester: &fakeRequester{}, body: `{"request":"x"}`, wantStatus: http.StatusUnauthorized},
		{name: "changes not configured", planner: &fakePlanner{}, password: testPassword, body: `{"request":"x"}`, wantStatus: http.StatusNotImplemented},
		{name: "bad json", planner: &fakePlanner{}, requester: &fakeRequester{}, password: testPassword, body: `nope`, wantStatus: http.StatusBadRequest},
		{name: "empty request", planner: &fakePlanner{}, requester: &fakeRequester{}, password: testPassword, body: `{"request":"  "}`, wantStatus: http.StatusBadRequest},
		{name: "too long", planner: &fakePlanner{}, requester: &fakeRequester{}, password: testPassword, body: `{"request":"` + long + `"}`, wantStatus: http.StatusBadRequest},
		{name: "planner fails", planner: &fakePlanner{err: errors.New("boom")}, requester: &fakeRequester{}, password: testPassword, body: `{"request":"x"}`, wantStatus: http.StatusBadGateway},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			recorder := postPlan(t, tt.planner, tt.requester, tt.password, tt.body)
			if recorder.Code != tt.wantStatus {
				t.Errorf("status = %d, want %d, body = %s", recorder.Code, tt.wantStatus, recorder.Body)
			}
		})
	}
}
