use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimit {
    requests: u32,
    window: Duration,
}

impl RateLimit {
    pub fn new(requests: u32, seconds: u64) -> Self {
        Self {
            requests,
            window: Duration::from_secs(seconds),
        }
    }
}

#[derive(Clone)]
struct RateLimitState {
    count: u32,
    window_start: Instant,
}

pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
    state: Arc<Mutex<HashMap<(u64, String), RateLimitState>>>, // (user_id, command) -> state
}

pub enum RateLimitResult {
    Allowed,
    Exceeded { seconds_remaining: u64 },
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_limit(&mut self, command: &str, limit: RateLimit) {
        self.limits.insert(command.to_string(), limit);
    }

    pub async fn check_rate_limit(&self, user_id: u64, command: &str) -> RateLimitResult {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        
        let limit = match self.limits.get(command) {
            Some(l) => l,
            None => return RateLimitResult::Allowed, // No limit set for this command
        };

        let key = (user_id, command.to_string());
        let current_state = state.entry(key).or_insert(RateLimitState {
            count: 0,
            window_start: now,
        });

        let time_passed = now.duration_since(current_state.window_start);

        // Reset window if needed
        if time_passed >= limit.window {
            current_state.count = 0;
            current_state.window_start = now;
        }

        // Check if limit is exceeded
        if current_state.count >= limit.requests {
            let seconds_remaining = (limit.window - time_passed).as_secs();
            return RateLimitResult::Exceeded { seconds_remaining };
        }

        // Increment counter
        current_state.count += 1;
        RateLimitResult::Allowed
    }
}
