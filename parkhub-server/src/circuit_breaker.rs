//! Hand-rolled circuit breaker for outbound HTTP calls.
//!
//! Prevents a flapping downstream endpoint from starving worker tasks: once a
//! threshold of consecutive failures is observed we "open" the breaker and
//! fail fast for a cooldown window before letting a single probe through.
//!
//! Three states:
//!   * [`State::Closed`] — all requests pass. Consecutive failures are counted;
//!     after `failure_threshold` we flip to [`State::Open`].
//!   * [`State::Open`] — every [`CircuitBreaker::call`] returns [`Error::Open`]
//!     immediately. After `reset_after` elapses we move to [`State::HalfOpen`].
//!   * [`State::HalfOpen`] — at most `half_open_max_calls` probes are allowed
//!     through. A single success closes the breaker, any failure re-opens it.
//!
//! A per-host registry ([`registry`]) ensures that one flapping webhook
//! operator doesn't trip every other destination. Breakers are keyed by URL
//! host (lower-cased); unknown/invalid URLs fall back to the literal URL.
//!
//! Metrics (reusing the existing `metrics` crate helpers):
//!   * `parkhub_circuit_breaker_state{name}` gauge — 0 = Closed, 1 = HalfOpen,
//!     2 = Open.
//!   * `parkhub_circuit_breaker_events_total{name, event}` counter — event is
//!     one of `success`, `failure`, `opened`, `half_opened`, `closed`,
//!     `short_circuit`, `rejected_half_open`.

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use metrics::{counter, gauge};

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Circuit-breaker configuration.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Consecutive failures in the Closed state before the breaker opens.
    pub failure_threshold: u32,
    /// How long the breaker stays Open before allowing a probe.
    pub reset_after: Duration,
    /// Max probe calls allowed concurrently in HalfOpen.
    pub half_open_max_calls: u32,
}

impl Config {
    /// Defaults tuned for webhook delivery: 5 consecutive failures, 30-second
    /// cooldown, one probe at a time in HalfOpen.
    #[must_use]
    pub const fn webhook_defaults() -> Self {
        Self {
            failure_threshold: 5,
            reset_after: Duration::from_secs(30),
            half_open_max_calls: 1,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::webhook_defaults()
    }
}

/// Current breaker state. Exposed for tests / metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Closed,
    Open,
    HalfOpen,
}

impl State {
    const fn as_gauge_value(self) -> f64 {
        match self {
            Self::Closed => 0.0,
            Self::HalfOpen => 1.0,
            Self::Open => 2.0,
        }
    }
}

/// Error returned by [`CircuitBreaker::call`].
#[derive(Debug)]
pub enum Error<E> {
    /// Breaker is open — call was short-circuited without touching the
    /// underlying resource.
    Open,
    /// Breaker is half-open and the probe quota is exhausted.
    HalfOpenRejected,
    /// The wrapped future returned an error.
    Inner(E),
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "circuit breaker open"),
            Self::HalfOpenRejected => write!(f, "circuit breaker half-open, probe quota exhausted"),
            Self::Inner(e) => write!(f, "{e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inner(e) => Some(e),
            Self::Open | Self::HalfOpenRejected => None,
        }
    }
}

/// Per-destination circuit breaker. Cheap to clone via `Arc`.
pub struct CircuitBreaker {
    name: String,
    config: Config,
    inner: Mutex<Inner>,
}

#[derive(Debug)]
struct Inner {
    state: State,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
    half_open_in_flight: u32,
}

impl CircuitBreaker {
    /// Create a new breaker with the given name (used for metric labels).
    #[must_use]
    pub fn new(name: impl Into<String>, config: Config) -> Arc<Self> {
        let name = name.into();
        let breaker = Arc::new(Self {
            name: name.clone(),
            config,
            inner: Mutex::new(Inner {
                state: State::Closed,
                consecutive_failures: 0,
                opened_at: None,
                half_open_in_flight: 0,
            }),
        });
        record_state(&name, State::Closed);
        breaker
    }

    /// Current state (snapshot). Primarily for tests + diagnostics.
    #[must_use]
    pub fn state(&self) -> State {
        self.inner.lock().expect("breaker mutex poisoned").state
    }

    /// Execute `f`, tracking success/failure for breaker bookkeeping.
    ///
    /// Returns [`Error::Open`] / [`Error::HalfOpenRejected`] without invoking
    /// `f` when the breaker short-circuits.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, Error<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        match self.try_acquire() {
            Ok(()) => {}
            Err(e) => return Err(e),
        }

        let result = f().await;
        match result {
            Ok(v) => {
                self.on_success();
                Ok(v)
            }
            Err(e) => {
                self.on_failure();
                Err(Error::Inner(e))
            }
        }
    }

    /// Check the current state and, if appropriate, reserve a probe slot.
    ///
    /// Returns `Err(Error::Open)` / `Err(Error::HalfOpenRejected)` when the
    /// call must be short-circuited. Otherwise `Ok(())` and the caller MUST
    /// eventually call [`on_success`] or [`on_failure`] to release the slot.
    fn try_acquire<E>(&self) -> Result<(), Error<E>> {
        let mut inner = self.inner.lock().expect("breaker mutex poisoned");
        match inner.state {
            State::Closed => Ok(()),
            State::Open => {
                // Maybe the cooldown elapsed — transition to HalfOpen.
                if let Some(opened_at) = inner.opened_at
                    && opened_at.elapsed() >= self.config.reset_after
                {
                    inner.state = State::HalfOpen;
                    inner.half_open_in_flight = 1;
                    record_state(&self.name, State::HalfOpen);
                    record_event(&self.name, "half_opened");
                    return Ok(());
                }
                record_event(&self.name, "short_circuit");
                Err(Error::Open)
            }
            State::HalfOpen => {
                if inner.half_open_in_flight >= self.config.half_open_max_calls {
                    record_event(&self.name, "rejected_half_open");
                    Err(Error::HalfOpenRejected)
                } else {
                    inner.half_open_in_flight += 1;
                    Ok(())
                }
            }
        }
    }

    fn on_success(&self) {
        let mut inner = self.inner.lock().expect("breaker mutex poisoned");
        match inner.state {
            State::Closed => {
                inner.consecutive_failures = 0;
            }
            State::HalfOpen => {
                // Probe succeeded — close the breaker.
                inner.state = State::Closed;
                inner.consecutive_failures = 0;
                inner.opened_at = None;
                inner.half_open_in_flight = 0;
                record_state(&self.name, State::Closed);
                record_event(&self.name, "closed");
            }
            State::Open => {
                // Unreachable in normal flow (try_acquire wouldn't have let us
                // through), but don't panic — just reset counters.
                inner.consecutive_failures = 0;
            }
        }
        record_event(&self.name, "success");
    }

    fn on_failure(&self) {
        let mut inner = self.inner.lock().expect("breaker mutex poisoned");
        record_event(&self.name, "failure");
        match inner.state {
            State::Closed => {
                inner.consecutive_failures = inner.consecutive_failures.saturating_add(1);
                if inner.consecutive_failures >= self.config.failure_threshold {
                    inner.state = State::Open;
                    inner.opened_at = Some(Instant::now());
                    inner.half_open_in_flight = 0;
                    record_state(&self.name, State::Open);
                    record_event(&self.name, "opened");
                }
            }
            State::HalfOpen => {
                // Probe failed — re-open.
                inner.state = State::Open;
                inner.opened_at = Some(Instant::now());
                inner.half_open_in_flight = 0;
                record_state(&self.name, State::Open);
                record_event(&self.name, "opened");
            }
            State::Open => {
                // Also unreachable in normal flow.
                inner.opened_at = Some(Instant::now());
            }
        }
    }
}

impl fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().ok();
        f.debug_struct("CircuitBreaker")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("state", &inner.as_ref().map(|i| i.state))
            .finish()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-host registry
// ─────────────────────────────────────────────────────────────────────────────

type Registry = Mutex<HashMap<String, Arc<CircuitBreaker>>>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Derive a registry key from a URL. Falls back to the raw URL when parsing
/// fails. Keys are lower-cased so `Example.com` and `example.com` share a
/// breaker.
#[must_use]
pub fn host_key(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_else(|| url.to_lowercase())
}

/// Fetch (or lazily create) the breaker for a given URL host.
#[must_use]
pub fn for_url(url: &str) -> Arc<CircuitBreaker> {
    for_key_with_config(&host_key(url), Config::webhook_defaults())
}

/// Fetch (or lazily create) a breaker for a custom key + config. Primarily
/// used by tests.
#[must_use]
pub fn for_key_with_config(key: &str, config: Config) -> Arc<CircuitBreaker> {
    let mut map = registry().lock().expect("breaker registry poisoned");
    map.entry(key.to_string())
        .or_insert_with(|| CircuitBreaker::new(key, config))
        .clone()
}

// ─────────────────────────────────────────────────────────────────────────────
// Metric helpers
// ─────────────────────────────────────────────────────────────────────────────

fn record_state(name: &str, state: State) {
    let labels = [("name", name.to_string())];
    gauge!("parkhub_circuit_breaker_state", &labels).set(state.as_gauge_value());
}

fn record_event(name: &str, event: &'static str) {
    let labels = [("name", name.to_string()), ("event", event.to_string())];
    counter!("parkhub_circuit_breaker_events_total", &labels).increment(1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn test_config() -> Config {
        Config {
            failure_threshold: 3,
            reset_after: Duration::from_millis(50),
            half_open_max_calls: 1,
        }
    }

    fn fresh(name: &str) -> Arc<CircuitBreaker> {
        // Use a unique name per test to avoid cross-test contamination via
        // the metrics recorder / registry.
        CircuitBreaker::new(format!("test-{name}-{}", uuid_like()), test_config())
    }

    fn uuid_like() -> u64 {
        use std::sync::atomic::AtomicU64;
        static N: AtomicU64 = AtomicU64::new(0);
        N.fetch_add(1, Ordering::Relaxed)
    }

    #[tokio::test]
    async fn closed_to_open_after_threshold_failures() {
        let breaker = fresh("closed_to_open");
        assert_eq!(breaker.state(), State::Closed);

        for _ in 0..3 {
            let res: Result<(), Error<&'static str>> = breaker.call(|| async { Err("boom") }).await;
            assert!(matches!(res, Err(Error::Inner(_))));
        }

        assert_eq!(breaker.state(), State::Open);

        // Subsequent calls short-circuit without invoking f.
        let called = AtomicU32::new(0);
        let res: Result<(), Error<&'static str>> = breaker
            .call(|| async {
                called.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .await;
        assert!(matches!(res, Err(Error::Open)));
        assert_eq!(called.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn open_to_half_open_after_reset() {
        let breaker = fresh("open_to_half_open");
        for _ in 0..3 {
            let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("boom") }).await;
        }
        assert_eq!(breaker.state(), State::Open);

        tokio::time::sleep(Duration::from_millis(80)).await;

        // First call after cooldown is a probe — state flips to HalfOpen
        // during try_acquire. The probe succeeds, which closes the breaker.
        let res: Result<(), Error<&'static str>> = breaker.call(|| async { Ok(()) }).await;
        assert!(res.is_ok());
        assert_eq!(breaker.state(), State::Closed);
    }

    #[tokio::test]
    async fn half_open_to_closed_on_successful_probe() {
        let breaker = fresh("half_open_to_closed");
        for _ in 0..3 {
            let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("boom") }).await;
        }
        tokio::time::sleep(Duration::from_millis(80)).await;

        let res: Result<u32, Error<&'static str>> = breaker.call(|| async { Ok(42_u32) }).await;
        assert_eq!(res.ok(), Some(42));
        assert_eq!(breaker.state(), State::Closed);
    }

    #[tokio::test]
    async fn half_open_to_open_on_failed_probe() {
        let breaker = fresh("half_open_to_open");
        for _ in 0..3 {
            let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("boom") }).await;
        }
        tokio::time::sleep(Duration::from_millis(80)).await;

        let res: Result<(), Error<&'static str>> =
            breaker.call(|| async { Err("still broken") }).await;
        assert!(matches!(res, Err(Error::Inner(_))));
        assert_eq!(breaker.state(), State::Open);

        // And we short-circuit again right away.
        let res: Result<(), Error<&'static str>> = breaker.call(|| async { Ok(()) }).await;
        assert!(matches!(res, Err(Error::Open)));
    }

    #[tokio::test]
    async fn half_open_rejects_extra_probes() {
        let breaker = fresh("half_open_rejects_extra");
        for _ in 0..3 {
            let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("boom") }).await;
        }
        tokio::time::sleep(Duration::from_millis(80)).await;

        // Manually take the probe slot by transitioning directly.
        {
            let mut inner = breaker.inner.lock().unwrap();
            inner.state = State::HalfOpen;
            inner.half_open_in_flight = 1;
        }
        let res: Result<(), Error<&'static str>> = breaker.call(|| async { Ok(()) }).await;
        assert!(matches!(res, Err(Error::HalfOpenRejected)));
    }

    #[tokio::test]
    async fn success_resets_consecutive_failures_in_closed() {
        let breaker = fresh("closed_reset");
        let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("e") }).await;
        let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("e") }).await;
        let _: Result<(), Error<&'static str>> = breaker.call(|| async { Ok(()) }).await;
        // Two more failures should NOT open the breaker if the counter reset.
        let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("e") }).await;
        let _: Result<(), Error<&'static str>> = breaker.call(|| async { Err("e") }).await;
        assert_eq!(breaker.state(), State::Closed);
    }

    #[test]
    fn host_key_lowercases_host() {
        assert_eq!(
            host_key("https://Hooks.Example.COM/path"),
            "hooks.example.com"
        );
    }

    #[test]
    fn host_key_falls_back_for_bad_urls() {
        assert_eq!(host_key("not a url"), "not a url");
    }

    #[test]
    fn registry_reuses_breakers_per_key() {
        let a = for_key_with_config("breaker.registry.test", test_config());
        let b = for_key_with_config("breaker.registry.test", test_config());
        assert!(Arc::ptr_eq(&a, &b));
    }
}
