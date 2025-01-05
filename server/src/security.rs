use actix_web::{
    error::ErrorUnauthorized,
    http::header::{self},
    Error, HttpRequest,
};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use parking_lot::RwLock;
use sha2::Sha256;
use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug)]
pub struct RateLimiter {
    requests: RwLock<HashMap<String, Vec<Instant>>>,
    window: Duration,
    max_requests: usize,
}

impl RateLimiter {
    pub fn new(window_secs: u64, max_requests: usize) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            window: Duration::from_secs(window_secs),
            max_requests,
        }
    }

    pub fn check_rate_limit(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write();

        requests.retain(|_, times| {
            times.retain(|&time| now.duration_since(time) <= self.window);
            !times.is_empty()
        });

        let times = requests.entry(ip.to_string()).or_default();
        if times.len() >= self.max_requests {
            false
        } else {
            times.push(now);
            true
        }
    }
}

pub fn generate_token(secret: &str) -> Result<String, Error> {
    let key: Hmac<Sha256> =
        Hmac::new_from_slice(secret.as_bytes()).map_err(|_| ErrorUnauthorized("Invalid key"))?;

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + 24 * 3600;

    let claims = HashMap::from([
        ("sub", Uuid::new_v4().to_string()),
        ("exp", expiration.to_string()),
    ]);

    claims
        .sign_with_key(&key)
        .map_err(|_| ErrorUnauthorized("Token generation failed"))
}

pub fn verify_token(token: &str, secret: &str) -> Result<HashMap<String, String>, Error> {
    let key: Hmac<Sha256> =
        Hmac::new_from_slice(secret.as_bytes()).map_err(|_| ErrorUnauthorized("Invalid key"))?;

    let claims: HashMap<String, String> = token
        .verify_with_key(&key)
        .map_err(|_| ErrorUnauthorized("Invalid token"))?;

    if let Some(exp) = claims.get("exp") {
        let exp: i64 = exp
            .parse()
            .map_err(|_| ErrorUnauthorized("Invalid expiration"))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        if exp < now {
            return Err(ErrorUnauthorized("Token expired"));
        }
    }

    Ok(claims)
}

pub fn validate_origin(req: &HttpRequest) -> Result<(), Error> {
    if let Some(origin) = req.headers().get(header::ORIGIN) {
        let origin_str = origin
            .to_str()
            .map_err(|_| ErrorUnauthorized("Invalid origin"))?;
        if !crate::config::CONFIG
            .allowed_origins
            .contains(&origin_str.to_string())
        {
            return Err(ErrorUnauthorized("Origin not allowed"));
        }
    }
    Ok(())
}

pub fn get_client_ip(req: &HttpRequest) -> String {
    req.connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string()
}
