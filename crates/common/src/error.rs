use thiserror::Error;

#[derive(Debug, Error)]
pub enum Layer7Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("WAF engine error: {0}")]
    WafEngine(String),

    #[error("rate limit exceeded for key: {0}")]
    RateLimited(String),

    #[error("IP blocked: {0}")]
    IpBlocked(String),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error("request blocked by WAF rule (status {status}): {message}")]
    WafBlocked { status: u16, message: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Layer7Result<T> = Result<T, Layer7Error>;
