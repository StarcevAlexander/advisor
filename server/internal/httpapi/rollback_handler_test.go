package httpapi

import (
	"errors"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"advisor/server/internal/changes"
)

func callRollback(t *testing.T, method, path string, requester changes.Requester, password string) *httptest.ResponseRecorder {
	t.Helper()
	handler := NewHandler(Options{
		Responder:      &fakeResponder{},
		Changes:        requester,
		Password:       testPassword,
		AllowedOrigins: []string{testOrigin},
		Logger:         slog.New(slog.NewTextHandler(io.Discard, nil)),
	})
	request := httptest.NewRequest(method, path, nil)
	if password != "" {
		request.Header.Set("Authorization", BearerPrefix+password)
	}
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	return recorder
}

func TestRollback(t *testing.T) {
	requester := &fakeRequester{}

	recorder := callRollback(t, http.MethodPost, "/api/rollback", requester, testPassword)

	if recorder.Code != http.StatusAccepted {
		t.Fatalf("status = %d, body = %s", recorder.Code, recorder.Body)
	}
	for _, want := range []string{`"id":"0123456789ab"`, `"url":"https://github.com/me/app/pulls"`} {
		if !strings.Contains(recorder.Body.String(), want) {
			t.Errorf("body %s lacks %s", recorder.Body, want)
		}
	}
	if requester.rollbacks != 1 {
		t.Errorf("rollbacks = %d, want 1", requester.rollbacks)
	}
}

func TestRollbackRejections(t *testing.T) {
	tests := []struct {
		name       string
		requester  changes.Requester
		password   string
		wantStatus int
	}{
		{name: "no password", requester: &fakeRequester{}, wantStatus: http.StatusUnauthorized},
		{name: "wrong password", requester: &fakeRequester{}, password: "nope-nope-nope", wantStatus: http.StatusUnauthorized},
		{name: "changes not configured", password: testPassword, wantStatus: http.StatusNotImplemented},
		{name: "github fails", requester: &fakeRequester{err: errors.New("boom")}, password: testPassword, wantStatus: http.StatusBadGateway},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			recorder := callRollback(t, http.MethodPost, "/api/rollback", tt.requester, tt.password)
			if recorder.Code != tt.wantStatus {
				t.Errorf("status = %d, want %d, body = %s", recorder.Code, tt.wantStatus, recorder.Body)
			}
		})
	}
}

func TestRollbackStatus(t *testing.T) {
	requester := &fakeRequester{status: changes.Status{State: changes.StateMerged, PullURL: "https://pr/9", PullNumber: 9}}

	recorder := callRollback(t, http.MethodGet, "/api/rollback/0123456789ab", requester, testPassword)

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, body = %s", recorder.Code, recorder.Body)
	}
	for _, want := range []string{`"state":"merged"`, `"pullNumber":9`} {
		if !strings.Contains(recorder.Body.String(), want) {
			t.Errorf("body %s lacks %s", recorder.Body, want)
		}
	}
	if len(requester.ids) != 1 || requester.ids[0] != "0123456789ab" {
		t.Errorf("ids = %v", requester.ids)
	}
}

func TestRollbackStatusRejections(t *testing.T) {
	tests := []struct {
		name       string
		requester  changes.Requester
		path       string
		password   string
		wantStatus int
	}{
		{name: "no password", requester: &fakeRequester{}, path: "/api/rollback/0123456789ab", wantStatus: http.StatusUnauthorized},
		{name: "bad id", requester: &fakeRequester{}, path: "/api/rollback/nope", password: testPassword, wantStatus: http.StatusBadRequest},
		{name: "changes not configured", path: "/api/rollback/0123456789ab", password: testPassword, wantStatus: http.StatusNotImplemented},
		{name: "github fails", requester: &fakeRequester{err: errors.New("boom")}, path: "/api/rollback/0123456789ab", password: testPassword, wantStatus: http.StatusBadGateway},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			recorder := callRollback(t, http.MethodGet, tt.path, tt.requester, tt.password)
			if recorder.Code != tt.wantStatus {
				t.Errorf("status = %d, want %d, body = %s", recorder.Code, tt.wantStatus, recorder.Body)
			}
		})
	}
}
