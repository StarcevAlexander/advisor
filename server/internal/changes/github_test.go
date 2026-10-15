package changes

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
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
	if !ValidID(receipt.ID) || gotBody.Inputs.ID != receipt.ID {
		t.Errorf("id: receipt=%q sent=%q", receipt.ID, gotBody.Inputs.ID)
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

const testID = "0123456789ab"

func statusServer(t *testing.T, runsJSON, pullsJSON string, pullsStatus int) *httptest.Server {
	t.Helper()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case strings.HasSuffix(r.URL.Path, "/runs"):
			_, _ = w.Write([]byte(runsJSON))
		case strings.HasSuffix(r.URL.Path, "/pulls"):
			if got := r.URL.Query().Get("head"); got != "me:claude/change-"+testID {
				t.Errorf("head = %q", got)
			}
			if pullsStatus != 0 {
				w.WriteHeader(pullsStatus)
			}
			_, _ = w.Write([]byte(pullsJSON))
		default:
			t.Errorf("unexpected path %s", r.URL.Path)
		}
	}))
	t.Cleanup(server.Close)
	return server
}

func runsJSON(title, status, conclusion string) string {
	return fmt.Sprintf(`{"workflow_runs":[{"display_title":"other","status":"completed","conclusion":"success","html_url":"https://run/other"},`+
		`{"display_title":%q,"status":%q,"conclusion":%q,"html_url":"https://run/mine"}]}`, title, status, conclusion)
}

func TestGitHubStatus(t *testing.T) {
	mine := RunTitlePrefix + testID
	tests := []struct {
		name       string
		runs       string
		pulls      string
		wantState  State
		wantPull   int
		wantRunURL string
	}{
		{name: "run not visible yet", runs: `{"workflow_runs":[]}`, pulls: `[]`, wantState: StateQueued},
		{name: "queued", runs: runsJSON(mine, "queued", ""), pulls: `[]`, wantState: StateQueued, wantRunURL: "https://run/mine"},
		{name: "in progress", runs: runsJSON(mine, "in_progress", ""), pulls: `[]`, wantState: StateWorking, wantRunURL: "https://run/mine"},
		{name: "failed", runs: runsJSON(mine, "completed", "failure"), pulls: `[]`, wantState: StateFailed, wantRunURL: "https://run/mine"},
		{name: "finished without a pull request", runs: runsJSON(mine, "completed", "success"), pulls: `[]`, wantState: StateNoChange, wantRunURL: "https://run/mine"},
		{
			name:       "open pull request",
			runs:       runsJSON(mine, "completed", "success"),
			pulls:      `[{"number":7,"html_url":"https://pr/7","state":"open","draft":false,"merged_at":null}]`,
			wantState:  StateReady,
			wantPull:   7,
			wantRunURL: "https://run/mine",
		},
		{
			name:       "draft pull request",
			runs:       runsJSON(mine, "completed", "success"),
			pulls:      `[{"number":8,"html_url":"https://pr/8","state":"open","draft":true,"merged_at":null}]`,
			wantState:  StateDraft,
			wantPull:   8,
			wantRunURL: "https://run/mine",
		},
		{
			name:       "merged pull request",
			runs:       runsJSON(mine, "completed", "success"),
			pulls:      `[{"number":9,"html_url":"https://pr/9","state":"closed","draft":false,"merged_at":"2026-10-16T00:00:00Z"}]`,
			wantState:  StateMerged,
			wantPull:   9,
			wantRunURL: "https://run/mine",
		},
		{
			name:       "closed pull request",
			runs:       runsJSON(mine, "completed", "success"),
			pulls:      `[{"number":10,"html_url":"https://pr/10","state":"closed","draft":false,"merged_at":null}]`,
			wantState:  StateClosed,
			wantPull:   10,
			wantRunURL: "https://run/mine",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			server := statusServer(t, tt.runs, tt.pulls, 0)
			github := NewGitHub(GitHubConfig{Token: "tok", Repo: "me/app", BaseURL: server.URL})

			status, err := github.Status(context.Background(), testID)
			if err != nil {
				t.Fatalf("Status: %v", err)
			}
			if status.State != tt.wantState || status.PullNumber != tt.wantPull || status.RunURL != tt.wantRunURL {
				t.Errorf("status = %+v", status)
			}
		})
	}
}

func TestGitHubStatusInvalidID(t *testing.T) {
	github := NewGitHub(GitHubConfig{Token: "tok"})

	for _, id := range []string{"", "short", "ZZZZZZZZZZZZ", "0123456789abc", "../../etc/passwd"} {
		if _, err := github.Status(context.Background(), id); !errors.Is(err, ErrInvalidID) {
			t.Errorf("id %q: err = %v, want ErrInvalidID", id, err)
		}
	}
}

func TestGitHubStatusLookupFailure(t *testing.T) {
	server := statusServer(t, runsJSON(RunTitlePrefix+testID, "completed", "success"), `{"message":"nope"}`, http.StatusInternalServerError)
	github := NewGitHub(GitHubConfig{Token: "tok", Repo: "me/app", BaseURL: server.URL})

	_, err := github.Status(context.Background(), testID)

	if !errors.Is(err, ErrLookupFailed) {
		t.Errorf("err = %v, want ErrLookupFailed", err)
	}
}

func TestNewIDIsValidAndUnique(t *testing.T) {
	first, err := newID()
	if err != nil {
		t.Fatal(err)
	}
	second, _ := newID()

	if !ValidID(first) || first == second {
		t.Errorf("ids: %q %q", first, second)
	}
}
