package changes

import (
	"bytes"
	"context"
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
	return &GitHub{config: config}
}

func (g *GitHub) Request(ctx context.Context, text string) (Receipt, error) {
	payload, err := json.Marshal(dispatchBody{
		Ref:    g.config.Branch,
		Inputs: dispatchInputs{Request: text},
	})
	if err != nil {
		return Receipt{}, fmt.Errorf("encode dispatch: %w", err)
	}

	url := fmt.Sprintf(DispatchPathFormat, strings.TrimRight(g.config.BaseURL, "/"), g.config.Repo, WorkflowFile)
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return Receipt{}, fmt.Errorf("build dispatch request: %w", err)
	}
	request.Header.Set("Authorization", "Bearer "+g.config.Token)
	request.Header.Set("Accept", "application/vnd.github+json")
	request.Header.Set("X-GitHub-Api-Version", APIVersion)
	request.Header.Set("Content-Type", "application/json")

	response, err := g.config.Client.Do(request)
	if err != nil {
		return Receipt{}, fmt.Errorf("send dispatch: %w", err)
	}
	defer response.Body.Close()

	if response.StatusCode != http.StatusNoContent {
		body, _ := io.ReadAll(io.LimitReader(response.Body, MaxErrorBodyBytes))
		return Receipt{}, fmt.Errorf("%w: status %d: %s", ErrDispatchRejected, response.StatusCode, strings.TrimSpace(string(body)))
	}
	return Receipt{PullsURL: fmt.Sprintf(PullsURLFormat, g.config.Repo)}, nil
}
