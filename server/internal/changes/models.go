package changes

import (
	"context"
	"errors"
	"net/http"
)

type State string

type Receipt struct {
	ID       string
	PullsURL string
}

type Status struct {
	State      State  `json:"state"`
	PullURL    string `json:"pullUrl,omitempty"`
	PullNumber int    `json:"pullNumber,omitempty"`
	RunURL     string `json:"runUrl,omitempty"`
}

type Requester interface {
	Request(ctx context.Context, text string) (Receipt, error)
	Status(ctx context.Context, id string) (Status, error)
	Rollback(ctx context.Context) (Receipt, error)
	RollbackStatus(ctx context.Context, id string) (Status, error)
}

type GitHubConfig struct {
	Token   string
	Repo    string
	Branch  string
	BaseURL string
	Client  *http.Client
}

type GitHub struct {
	config GitHubConfig
}

type dispatchInputs struct {
	Request string `json:"request"`
	ID      string `json:"id"`
}

type rollbackInputs struct {
	ID string `json:"id"`
}

type dispatchBody struct {
	Ref    string `json:"ref"`
	Inputs any    `json:"inputs"`
}

type trackedWorkflow struct {
	file         string
	runPrefix    string
	branchPrefix string
}

type workflowRun struct {
	DisplayTitle string `json:"display_title"`
	Status       string `json:"status"`
	Conclusion   string `json:"conclusion"`
	HTMLURL      string `json:"html_url"`
}

type workflowRunsResponse struct {
	WorkflowRuns []workflowRun `json:"workflow_runs"`
}

type pullRequest struct {
	Number   int     `json:"number"`
	HTMLURL  string  `json:"html_url"`
	State    string  `json:"state"`
	Draft    bool    `json:"draft"`
	MergedAt *string `json:"merged_at"`
}

var (
	ErrDispatchRejected = errors.New("github rejected the dispatch")
	ErrLookupFailed     = errors.New("github lookup failed")
	ErrInvalidID        = errors.New("invalid change id")
)
