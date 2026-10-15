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
)

type fakeRequester struct {
	err   error
	texts []string
}

func (f *fakeRequester) Request(_ context.Context, text string) (changes.Receipt, error) {
	f.texts = append(f.texts, text)
	return changes.Receipt{PullsURL: "https://github.com/me/app/pulls"}, f.err
}

func postChange(t *testing.T, requester changes.Requester, password, body string) *httptest.ResponseRecorder {
	t.Helper()
	handler := NewHandler(Options{
		Responder:      &fakeResponder{},
		Changes:        requester,
		Password:       testPassword,
		AllowedOrigins: []string{testOrigin},
		Logger:         slog.New(slog.NewTextHandler(io.Discard, nil)),
	})
	request := httptest.NewRequest(http.MethodPost, "/api/change", strings.NewReader(body))
	request.Header.Set("Content-Type", "application/json")
	if password != "" {
		request.Header.Set("Authorization", BearerPrefix+password)
	}
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	return recorder
}

func TestChangeAccepted(t *testing.T) {
	requester := &fakeRequester{}

	recorder := postChange(t, requester, testPassword, `{"request":"  перекрасить кнопку  "}`)

	if recorder.Code != http.StatusAccepted {
		t.Fatalf("status = %d, body = %s", recorder.Code, recorder.Body)
	}
	if len(requester.texts) != 1 || requester.texts[0] != "перекрасить кнопку" {
		t.Errorf("texts = %v", requester.texts)
	}
	if !strings.Contains(recorder.Body.String(), "https://github.com/me/app/pulls") {
		t.Errorf("body = %s", recorder.Body)
	}
}

func TestChangeRejections(t *testing.T) {
	tests := []struct {
		name       string
		requester  changes.Requester
		password   string
		body       string
		wantStatus int
		wantCalls  int
	}{
		{name: "no password", requester: &fakeRequester{}, password: "", body: `{"request":"x"}`, wantStatus: http.StatusUnauthorized},
		{name: "not configured", requester: nil, password: testPassword, body: `{"request":"x"}`, wantStatus: http.StatusNotImplemented},
		{name: "bad json", requester: &fakeRequester{}, password: testPassword, body: `{`, wantStatus: http.StatusBadRequest},
		{name: "empty request", requester: &fakeRequester{}, password: testPassword, body: `{"request":"  "}`, wantStatus: http.StatusBadRequest},
		{
			name:       "too long",
			requester:  &fakeRequester{},
			password:   testPassword,
			body:       `{"request":"` + strings.Repeat("я", MaxChangeRequestRunes+1) + `"}`,
			wantStatus: http.StatusBadRequest,
		},
		{
			name:       "github failure",
			requester:  &fakeRequester{err: errors.New("boom")},
			password:   testPassword,
			body:       `{"request":"x"}`,
			wantStatus: http.StatusBadGateway,
			wantCalls:  1,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			recorder := postChange(t, tt.requester, tt.password, tt.body)
			if recorder.Code != tt.wantStatus {
				t.Errorf("status = %d, want %d, body = %s", recorder.Code, tt.wantStatus, recorder.Body)
			}
			if fake, ok := tt.requester.(*fakeRequester); ok && len(fake.texts) != tt.wantCalls {
				t.Errorf("calls = %d, want %d", len(fake.texts), tt.wantCalls)
			}
			if strings.Contains(recorder.Body.String(), "boom") {
				t.Errorf("internal error leaked: %s", recorder.Body)
			}
		})
	}
}
