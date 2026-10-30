package config

const (
	EnvPort           = "PORT"
	EnvAPIKey         = "ZAI_API_KEY"
	EnvBaseURL        = "ZAI_BASE_URL"
	EnvAccessPassword = "ACCESS_PASSWORD"
	EnvAllowedOrigins = "ALLOWED_ORIGINS"
	EnvModel          = "ADVISOR_MODEL"
	EnvGitHubToken    = "GITHUB_TOKEN"
	EnvGitHubRepo     = "GITHUB_REPO"
	EnvGitHubBranch   = "GITHUB_BRANCH"
	EnvGitHubAPIURL   = "GITHUB_API_URL"
	EnvDatabaseURL    = "DATABASE_URL"
	DefaultPort       = "8080"
	DefaultBaseURL    = "https://api.z.ai/api/anthropic"
	DefaultModel      = "glm-5.3"
	MinPasswordLength = 8
	DotEnvFileName    = ".env"
)

var DefaultAllowedOrigins = []string{
	"https://starcevalexander.github.io",
	"http://localhost:4200",
	"http://127.0.0.1:4200",
}
