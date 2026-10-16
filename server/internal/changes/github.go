package changes

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

func NewGitHub(config GitHubConfig) *GitHub {
	if config.BaseURL == "" {
		config.BaseURL = DefaultAPIBaseURL
	}
	if config.Repo == "" {
		config.Repo = DefaultRepo
	}
	if config.Branch == "" {
		config.Branch = DefaultBranch
	}
	if config.Client == nil {
		config.Client = &http.Client{Timeout: RequestTimeout}
	}
	config.BaseURL = strings.TrimRight(config.BaseURL, "/")
	return &GitHub{config: config}
}

var (
	changeWorkflow   = trackedWorkflow{file: WorkflowFile, runPrefix: RunTitlePrefix, branchPrefix: BranchPrefix}
	rollbackWorkflow = trackedWorkflow{file: RollbackWorkflow, runPrefix: RollbackRunPrefix, branchPrefix: RollbackBranchHead}
)

func (g *GitHub) Request(ctx context.Context, text string) (Receipt, error) {
	id, err := newID()
	if err != nil {
		return Receipt{}, err
	}
	return g.dispatch(ctx, changeWorkflow, id, dispatchInputs{Request: text, ID: id})
}

func (g *GitHub) Rollback(ctx context.Context) (Receipt, error) {
	id, err := newID()
	if err != nil {
		return Receipt{}, err
	}
	return g.dispatch(ctx, rollbackWorkflow, id, rollbackInputs{ID: id})
}

func (g *GitHub) dispatch(ctx context.Context, workflow trackedWorkflow, id string, inputs any) (Receipt, error) {
	payload, err := json.Marshal(dispatchBody{Ref: g.config.Branch, Inputs: inputs})
	if err != nil {
		return Receipt{}, fmt.Errorf("encode dispatch: %w", err)
	}

	url := fmt.Sprintf(DispatchPathFormat, g.config.BaseURL, g.config.Repo, workflow.file)
	request, err := g.newRequest(ctx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return Receipt{}, err
	}
	request.Header.Set("Content-Type", "application/json")

	response, err := g.config.Client.Do(request)
	if err != nil {
		return Receipt{}, fmt.Errorf("send dispatch: %w", err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusNoContent {
		return Receipt{}, fmt.Errorf("%w: %w", ErrDispatchRejected, statusError(response))
	}
	return Receipt{ID: id, PullsURL: fmt.Sprintf(PullsURLFormat, g.config.Repo)}, nil
}

func (g *GitHub) Status(ctx context.Context, id string) (Status, error) {
	return g.status(ctx, changeWorkflow, id)
}

func (g *GitHub) RollbackStatus(ctx context.Context, id string) (Status, error) {
	return g.status(ctx, rollbackWorkflow, id)
}

func (g *GitHub) status(ctx context.Context, workflow trackedWorkflow, id string) (Status, error) {
	if !ValidID(id) {
		return Status{}, ErrInvalidID
	}

	var runs workflowRunsResponse
	runsURL := fmt.Sprintf(RunsPathFormat, g.config.BaseURL, g.config.Repo, workflow.file)
	if err := g.getJSON(ctx, runsURL, &runs); err != nil {
		return Status{}, err
	}

	run := findRun(runs.WorkflowRuns, workflow.runPrefix+id)
	status, finishedOK := statusFromRun(run)
	if !finishedOK {
		return status, nil
	}

	owner, _, _ := strings.Cut(g.config.Repo, "/")
	var pulls []pullRequest
	pullsURL := fmt.Sprintf(PullsPathFormat, g.config.BaseURL, g.config.Repo, owner, workflow.branchPrefix+id)
	if err := g.getJSON(ctx, pullsURL, &pulls); err != nil {
		return Status{}, err
	}
	return statusFromPull(status, pulls), nil
}

func findRun(runs []workflowRun, title string) *workflowRun {
	for i := range runs {
		if runs[i].DisplayTitle == title {
			return &runs[i]
		}
	}
	return nil
}

func statusFromRun(run *workflowRun) (Status, bool) {
	switch {
	case run == nil:
		return Status{State: StateQueued}, false
	case run.Status != "completed":
		state := StateWorking
		if run.Status == "queued" || run.Status == "waiting" || run.Status == "pending" {
			state = StateQueued
		}
		return Status{State: state, RunURL: run.HTMLURL}, false
	case run.Conclusion != "success":
		return Status{State: StateFailed, RunURL: run.HTMLURL}, false
	default:
		return Status{State: StateNoChange, RunURL: run.HTMLURL}, true
	}
}

func statusFromPull(base Status, pulls []pullRequest) Status {
	if len(pulls) == 0 {
		return base
	}
	pull := pulls[0]
	base.PullURL = pull.HTMLURL
	base.PullNumber = pull.Number
	switch {
	case pull.MergedAt != nil:
		base.State = StateMerged
	case pull.State == "closed":
		base.State = StateClosed
	case pull.Draft:
		base.State = StateDraft
	default:
		base.State = StateReady
	}
	return base
}

func (g *GitHub) getJSON(ctx context.Context, url string, out any) error {
	request, err := g.newRequest(ctx, http.MethodGet, url, nil)
	if err != nil {
		return err
	}
	response, err := g.config.Client.Do(request)
	if err != nil {
		return fmt.Errorf("%w: %w", ErrLookupFailed, err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		return fmt.Errorf("%w: %w", ErrLookupFailed, statusError(response))
	}
	if err := json.NewDecoder(response.Body).Decode(out); err != nil {
		return fmt.Errorf("%w: decode: %w", ErrLookupFailed, err)
	}
	return nil
}

func (g *GitHub) newRequest(ctx context.Context, method, url string, body io.Reader) (*http.Request, error) {
	request, err := http.NewRequestWithContext(ctx, method, url, body)
	if err != nil {
		return nil, fmt.Errorf("build request: %w", err)
	}
	request.Header.Set("Authorization", "Bearer "+g.config.Token)
	request.Header.Set("Accept", "application/vnd.github+json")
	request.Header.Set("X-GitHub-Api-Version", APIVersion)
	return request, nil
}

func statusError(response *http.Response) error {
	body, _ := io.ReadAll(io.LimitReader(response.Body, MaxErrorBodyBytes))
	return fmt.Errorf("status %d: %s", response.StatusCode, strings.TrimSpace(string(body)))
}

func newID() (string, error) {
	buffer := make([]byte, IDBytes)
	if _, err := rand.Read(buffer); err != nil {
		return "", fmt.Errorf("generate id: %w", err)
	}
	return hex.EncodeToString(buffer), nil
}
