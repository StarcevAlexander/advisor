package changes

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestGitHubRequest(t *testing.T) {
	var gotPath, gotAuth, gotVersion string
	var gotBody dispatchBody
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		gotAuth = r.Header.Get("Authorization")
		gotVersion = r.Header.Get("X-GitHub-Api-Version")
		body, _ := io.ReadAll(r.Body)
		if err := json.Unmarshal(body, &gotBody); err != nil {
			t.Errorf("decode: %v", err)
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	t.Cleanup(server.Close)

	github := NewGitHub(GitHubConfig{Token: "tok", Repo: "me/app", Branch: "main", BaseURL: server.URL})
	receipt, err := github.Request(context.Background(), "перекрасить кнопку")
	if err != nil {
		t.Fatalf("Request: %v", err)
	}

	if gotPath != "/repos/me/app/actions/workflows/claude-change.yml/dispatches" {
		t.Errorf("path = %q", gotPath)
	}
	if gotAuth != "Bearer tok" || gotVersion != APIVersion {
		t.Errorf("headers: auth=%q version=%q", gotAuth, gotVersion)
	}
	if gotBody.Ref != "main" || gotBody.Inputs.Request != "перекрасить кнопку" {
		t.Errorf("body = %+v", gotBody)
	}
	if receipt.PullsURL != "https://github.com/me/app/pulls" {
		t.Errorf("pulls url = %q", receipt.PullsURL)
	}
}

func TestGitHubRequestRejected(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusForbidden)
		_, _ = w.Write([]byte(`{"message":"Resource not accessible by personal access token"}`))
	}))
	t.Cleanup(server.Close)

	github := NewGitHub(GitHubConfig{Token: "tok", BaseURL: server.URL})
	_, err := github.Request(context.Background(), "x")

	if !errors.Is(err, ErrDispatchRejected) {
		t.Fatalf("err = %v, want ErrDispatchRejected", err)
	}
	if !strings.Contains(err.Error(), "403") {
		t.Errorf("status missing from error: %v", err)
	}
}

func TestNewGitHubDefaults(t *testing.T) {
	github := NewGitHub(GitHubConfig{Token: "tok"})

	if github.config.Repo != DefaultRepo || github.config.Branch != DefaultBranch || github.config.BaseURL != DefaultAPIBaseURL {
		t.Errorf("config = %+v", github.config)
	}
}
