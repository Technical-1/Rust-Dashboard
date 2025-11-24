use thiserror::Error;

/// Errors that can occur in the Rust Dashboard application
#[derive(Error, Debug)]
pub enum DashboardError {
    /// Mutex lock was poisoned (another thread panicked while holding the lock)
    #[error("Mutex lock was poisoned: {0}")]
    MutexPoisoned(String),

    /// Failed to acquire mutex lock
    #[error("Failed to acquire mutex lock: {0}")]
    MutexLockFailed(String),

    /// System refresh failed
    #[error("System refresh failed: {0}")]
    SystemRefreshFailed(String),
}

impl<T> From<std::sync::PoisonError<T>> for DashboardError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        DashboardError::MutexPoisoned(err.to_string())
    }
}
