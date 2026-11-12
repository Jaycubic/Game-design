//! Small in-memory sliding-window rate limiter for authentication and room joins.

use crate::error::ApiError;
use actix_web::HttpRequest;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    window: Duration,
    hits: Mutex<HashMap<(IpAddr, &'static str), VecDeque<Instant>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        RateLimiter { window: Duration::from_secs(60), hits: Mutex::new(HashMap::new()) }
    }
}

fn limit_for(bucket: &str) -> usize {
    match bucket {
        "auth" => 10,
        "join" => 20,
        _ => 60,
    }
}

impl RateLimiter {
    pub fn check(&self, req: &HttpRequest, bucket: &'static str) -> Result<(), ApiError> {
        let ip = req.peer_addr().map(|a| a.ip()).unwrap_or(IpAddr::from([0, 0, 0, 0]));
        self.check_ip(ip, bucket)
    }

    pub fn check_ip(&self, ip: IpAddr, bucket: &'static str) -> Result<(), ApiError> {
        let now = Instant::now();
        let mut map = self.hits.lock().expect("rate limiter lock");
        if map.len() > 10_000 {
            map.retain(|_, q| q.back().is_some_and(|t| now.duration_since(*t) < self.window));
        }
        let q = map.entry((ip, bucket)).or_default();
        while q.front().is_some_and(|t| now.duration_since(*t) >= self.window) {
            q.pop_front();
        }
        if q.len() >= limit_for(bucket) {
            return Err(ApiError::too_many());
        }
        q.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_limit() {
        let rl = RateLimiter::default();
        let ip = IpAddr::from([10, 0, 0, 1]);
        for _ in 0..10 {
            rl.check_ip(ip, "auth").unwrap();
        }
        assert!(rl.check_ip(ip, "auth").is_err());
        assert!(rl.check_ip(IpAddr::from([10, 0, 0, 2]), "auth").is_ok(), "limits are per address");
    }
}
