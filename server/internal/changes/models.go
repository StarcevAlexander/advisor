package changes

import (
	"context"
	"errors"
	"net/http"
)

type Receipt struct {
	PullsURL string
}

type Requester interface {
	Request(ctx context.Context, text string) (Receipt, error)
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
}

type dispatchBody struct {
	Ref    string         `json:"ref"`
	Inputs dispatchInputs `json:"inputs"`
}

var ErrDispatchRejected = errors.New("github rejected the dispatch")
