package changes

import "time"

const (
	WorkflowFile       = "claude-change.yml"
	DefaultAPIBaseURL  = "https://api.github.com"
	DefaultRepo        = "StarcevAlexander/advisor"
	DefaultBranch      = "master"
	APIVersion         = "2022-11-28"
	RequestTimeout     = 15 * time.Second
	MaxErrorBodyBytes  = 512
	PullsURLFormat     = "https://github.com/%s/pulls"
	DispatchPathFormat = "%s/repos/%s/actions/workflows/%s/dispatches"
)
