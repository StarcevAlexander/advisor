package changes

import (
	"regexp"
	"time"
)

const (
	WorkflowFile       = "claude-change.yml"
	RunTitlePrefix     = "Claude change "
	BranchPrefix       = "claude/change-"
	DefaultAPIBaseURL  = "https://api.github.com"
	DefaultRepo        = "StarcevAlexander/advisor"
	DefaultBranch      = "master"
	APIVersion         = "2022-11-28"
	RequestTimeout     = 15 * time.Second
	MaxErrorBodyBytes  = 512
	IDBytes            = 6
	PullsURLFormat     = "https://github.com/%s/pulls"
	DispatchPathFormat = "%s/repos/%s/actions/workflows/%s/dispatches"
	RunsPathFormat     = "%s/repos/%s/actions/workflows/%s/runs?event=workflow_dispatch&per_page=30"
	PullsPathFormat    = "%s/repos/%s/pulls?state=all&head=%s:%s"
)

const (
	StateQueued   State = "queued"
	StateWorking  State = "working"
	StateReady    State = "ready"
	StateDraft    State = "draft"
	StateMerged   State = "merged"
	StateClosed   State = "closed"
	StateFailed   State = "failed"
	StateNoChange State = "nochange"
)

var idPattern = regexp.MustCompile(`^[a-f0-9]{12}$`)

func ValidID(id string) bool {
	return idPattern.MatchString(id)
}
