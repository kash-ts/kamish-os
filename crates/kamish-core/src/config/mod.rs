use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_level: String,
    pub env: String,
    pub ai_cli_path: String,
    pub ai_cli_enabled: bool,
}

impl Config {
    pub fn from_env() -> Self {
        // Attempt to load .env file if available
        let _ = dotenvy::dotenv();

        let host = env::var("KAMISH_HOST")
            .or_else(|_| env::var("HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("KAMISH_PORT")
            .or_else(|_| env::var("PORT"))
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://kamish_core.db?mode=rwc".to_string());

        let log_level = env::var("RUST_LOG")
            .unwrap_or_else(|_| "info,kamish_core=debug,tower_http=info".to_string());

        let env = env::var("KAMISH_ENV").unwrap_or_else(|_| "development".to_string());

        let ai_cli_path = env::var("AI_CLI_PATH").unwrap_or_else(|_| "agy".to_string());
        let ai_cli_enabled = env::var("AI_CLI_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        Self {
            host,
            port,
            database_url,
            log_level,
            env,
            ai_cli_path,
            ai_cli_enabled,
        }
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::from_env();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(config.database_url.starts_with("sqlite://"));
        assert_eq!(config.ai_cli_path, "agy");
        assert!(config.ai_cli_enabled);
        assert_eq!(config.server_addr(), "0.0.0.0:8080");
    }
}
