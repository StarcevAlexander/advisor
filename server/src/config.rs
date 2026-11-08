use std::collections::HashMap;

pub const ENV_PORT: &str = "PORT";
pub const ENV_API_KEY: &str = "ZAI_API_KEY";
pub const ENV_BASE_URL: &str = "ZAI_BASE_URL";
pub const ENV_ACCESS_PASSWORD: &str = "ACCESS_PASSWORD";
pub const ENV_ALLOWED_ORIGINS: &str = "ALLOWED_ORIGINS";
pub const ENV_MODEL: &str = "ADVISOR_MODEL";
pub const ENV_VISION_MODEL: &str = "ADVISOR_VISION_MODEL";
pub const ENV_GITHUB_TOKEN: &str = "GITHUB_TOKEN";
pub const ENV_GITHUB_REPO: &str = "GITHUB_REPO";
pub const ENV_GITHUB_BRANCH: &str = "GITHUB_BRANCH";
pub const ENV_GITHUB_API_URL: &str = "GITHUB_API_URL";
pub const ENV_DATABASE_URL: &str = "DATABASE_URL";

pub const DEFAULT_PORT: &str = "8080";
pub const DEFAULT_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
pub const DEFAULT_MODEL: &str = "glm-5.3";
pub const DEFAULT_VISION_MODEL: &str = "glm-5.3-flash";
pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const DOT_ENV_FILE_NAME: &str = ".env";

pub const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "https://starcevalexander.github.io",
    "http://localhost:4200",
    "http://127.0.0.1:4200",
];

pub type DotEnv = HashMap<String, String>;

#[derive(Debug)]
pub struct Config {
    pub port: String,
    pub api_key: String,
    pub base_url: String,
    pub access_password: String,
    pub allowed_origins: Vec<String>,
    pub model: String,
    pub vision_model: String,
    pub github_token: String,
    pub github_repo: String,
    pub github_branch: String,
    pub github_api_url: String,
    pub database_url: String,
}

pub fn load(dotenv: &DotEnv) -> Result<Config, String> {
    let value = |name: &str| {
        std::env::var(name)
            .ok()
            .or_else(|| dotenv.get(name).cloned())
    };
    let trimmed = |name: &str| {
        value(name)
            .map(|v| v.trim().to_string())
            .unwrap_or_default()
    };

    let cfg = Config {
        port: value_or(&value(ENV_PORT).unwrap_or_default(), DEFAULT_PORT),
        api_key: trimmed(ENV_API_KEY),
        base_url: value_or(&trimmed(ENV_BASE_URL), DEFAULT_BASE_URL),
        access_password: value(ENV_ACCESS_PASSWORD).unwrap_or_default(),
        allowed_origins: parse_list(&value(ENV_ALLOWED_ORIGINS).unwrap_or_default()),
        model: value_or(&value(ENV_MODEL).unwrap_or_default(), DEFAULT_MODEL),
        vision_model: value_or(
            &value(ENV_VISION_MODEL).unwrap_or_default(),
            DEFAULT_VISION_MODEL,
        ),
        github_token: trimmed(ENV_GITHUB_TOKEN),
        github_repo: trimmed(ENV_GITHUB_REPO),
        github_branch: trimmed(ENV_GITHUB_BRANCH),
        github_api_url: trimmed(ENV_GITHUB_API_URL),
        database_url: trimmed(ENV_DATABASE_URL),
    };

    let mut problems: Vec<String> = Vec::new();
    if cfg.api_key.is_empty() {
        problems.push(format!("{ENV_API_KEY} is not set"));
    }
    if cfg.access_password.chars().count() < MIN_PASSWORD_LENGTH {
        problems.push(format!(
            "{ENV_ACCESS_PASSWORD} must be at least {MIN_PASSWORD_LENGTH} characters"
        ));
    }
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(cfg)
}

pub fn load_dot_env(path: &str) -> DotEnv {
    let mut map = DotEnv::new();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in contents.lines() {
        if let Some((key, value)) = parse_dot_env_line(line) {
            map.entry(key).or_insert(value);
        }
    }
    map
}

pub fn parse_dot_env_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let key = key.strip_prefix("export ").unwrap_or(key).trim();
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value);
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

pub fn parse_list(raw: &str) -> Vec<String> {
    let items: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    if items.is_empty() {
        return DEFAULT_ALLOWED_ORIGINS
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
    items
}

pub fn value_or(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> DotEnv {
        DotEnv::new()
    }

    #[test]
    fn load_requires_api_key_and_password() {
        let err = load(&env()).unwrap_err();
        assert!(err.contains(ENV_API_KEY));
        assert!(err.contains(ENV_ACCESS_PASSWORD));
    }

    #[test]
    fn load_accepts_valid_config() {
        let mut dotenv = env();
        dotenv.insert(ENV_API_KEY.to_string(), "key".to_string());
        dotenv.insert(ENV_ACCESS_PASSWORD.to_string(), "longenough".to_string());
        let cfg = load(&dotenv).unwrap();
        assert_eq!(cfg.port, DEFAULT_PORT);
        assert_eq!(cfg.base_url, DEFAULT_BASE_URL);
        assert_eq!(cfg.model, DEFAULT_MODEL);
        assert_eq!(cfg.vision_model, DEFAULT_VISION_MODEL);
        assert_eq!(cfg.allowed_origins.len(), 3);
    }

    #[test]
    fn load_overrides_defaults_from_dotenv() {
        let mut dotenv = env();
        dotenv.insert(ENV_API_KEY.to_string(), "key".to_string());
        dotenv.insert(ENV_ACCESS_PASSWORD.to_string(), "longenough".to_string());
        dotenv.insert(ENV_PORT.to_string(), "9000".to_string());
        dotenv.insert(ENV_MODEL.to_string(), "custom-model".to_string());
        let cfg = load(&dotenv).unwrap();
        assert_eq!(cfg.port, "9000");
        assert_eq!(cfg.model, "custom-model");
    }

    #[test]
    fn parse_line_handles_quotes_comments_and_export() {
        assert_eq!(
            parse_dot_env_line("  KEY = \"value\" "),
            Some(("KEY".to_string(), "value".to_string()))
        );
        assert_eq!(
            parse_dot_env_line("export OTHER='quoted'"),
            Some(("OTHER".to_string(), "quoted".to_string()))
        );
        assert_eq!(parse_dot_env_line("# comment"), None);
        assert_eq!(parse_dot_env_line(""), None);
        assert_eq!(parse_dot_env_line("NOEQUALS"), None);
    }

    #[test]
    fn parse_list_falls_back_to_defaults() {
        assert_eq!(parse_list("").len(), 3);
        assert_eq!(parse_list(" , ,").len(), 3);
        let list = parse_list(" a , b ");
        assert_eq!(list, vec!["a".to_string(), "b".to_string()]);
    }
}
