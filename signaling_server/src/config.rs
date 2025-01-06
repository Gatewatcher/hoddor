use once_cell::sync::Lazy;
use std::env;

pub struct Config {
    pub allowed_origins: Vec<String>,
    pub jwt_secret: String,
    pub port: u16,
    pub max_connections: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string(),
                "https://localhost:5173".to_string(),
                "https://127.0.0.1:5173".to_string(),
                "null".to_string(),
            ],
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            max_connections: env::var("MAX_CONNECTIONS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
        }
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(Config::default);
