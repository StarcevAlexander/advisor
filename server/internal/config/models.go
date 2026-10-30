package config

type Config struct {
	Port           string
	APIKey         string
	BaseURL        string
	AccessPassword string
	AllowedOrigins []string
	Model          string
	GitHubToken    string
	GitHubRepo     string
	GitHubBranch   string
	GitHubAPIURL   string
	DatabaseURL    string
}
