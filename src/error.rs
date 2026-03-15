use serde_json::Value;
use std::error::Error as StdError;
use std::time::Duration;
use thiserror::Error;

type BoxError = Box<dyn StdError + Send + Sync>;

/// Result alias used by the Conduit Rust SDK.
pub type Result<T> = std::result::Result<T, ConduitError>;

#[derive(Debug, Error)]
#[error("{message}")]
/// Shared error context used by several SDK error variants.
pub struct ErrorContext {
    message: String,
    code: String,
    request_id: Option<String>,
    #[source]
    source: Option<BoxError>,
}

impl ErrorContext {
    fn new(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
            request_id: None,
            source: None,
        }
    }

    fn with_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }

    fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
/// Source-specific context used by upload and remote fetch errors.
pub struct SourceContext {
    message: String,
    code: String,
    request_id: Option<String>,
    url: Option<String>,
    status: Option<u16>,
    #[source]
    source: Option<BoxError>,
}

impl SourceContext {
    fn new(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
            request_id: None,
            url: None,
            status: None,
            source: None,
        }
    }

    fn with_remote(mut self, url: Option<String>, status: Option<u16>) -> Self {
        self.url = url;
        self.status = status;
        self
    }

    fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
/// API error context shared by auth, validation, and generic API failures.
pub struct ApiContext {
    message: String,
    code: String,
    request_id: Option<String>,
    status: u16,
    details: Option<Box<Value>>,
}

impl ApiContext {
    fn new(
        status: u16,
        request_id: Option<String>,
        message: impl Into<String>,
        code: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
            request_id,
            status,
            details: details.map(Box::new),
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
/// Rate-limit error context.
pub struct RateLimitContext {
    message: String,
    code: String,
    request_id: Option<String>,
    status: u16,
    details: Option<Box<Value>>,
    retry_after: Option<Duration>,
}

#[derive(Debug, Error)]
#[error("{message}")]
/// Insufficient credits error context.
pub struct CreditsContext {
    message: String,
    code: String,
    request_id: Option<String>,
    status: u16,
    details: Option<Box<Value>>,
    required: f64,
    available: f64,
}

#[derive(Debug, Error)]
#[error("{message}")]
/// Job failure or cancellation context.
pub struct JobContext {
    message: String,
    code: String,
    request_id: Option<String>,
    job_id: String,
}

#[derive(Debug, Error)]
#[error("{message}")]
/// Stream polling error context.
pub struct StreamContext {
    message: String,
    code: String,
    request_id: Option<String>,
    job_id: Option<String>,
    last_event_id: Option<String>,
    retry_count: usize,
    #[source]
    source: Option<BoxError>,
}

impl StreamContext {
    fn new(message: impl Into<String>, job_id: Option<String>) -> Self {
        Self {
            message: message.into(),
            code: "stream_error".into(),
            request_id: None,
            job_id,
            last_event_id: None,
            retry_count: 0,
            source: None,
        }
    }

    fn with_source<E>(mut self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }
}

#[derive(Debug, Error)]
/// Typed error returned by all public SDK operations.
pub enum ConduitError {
    #[error(transparent)]
    /// Generic SDK error.
    Base(Box<ErrorContext>),
    #[error(transparent)]
    /// Client initialization or configuration error.
    Initialization(Box<ErrorContext>),
    #[error(transparent)]
    /// Unsupported runtime capability error.
    UnsupportedRuntime(Box<ErrorContext>),
    #[error(transparent)]
    /// Webhook signature verification error.
    WebhookVerification(Box<ErrorContext>),
    #[error(transparent)]
    /// Invalid local source configuration error.
    InvalidSource(Box<SourceContext>),
    #[error(transparent)]
    /// Remote source fetch error.
    RemoteFetch(Box<SourceContext>),
    #[error(transparent)]
    /// Remote source fetch timeout error.
    RemoteFetchTimeout(Box<SourceContext>),
    #[error(transparent)]
    /// Remote source exceeded the upload size limit.
    RemoteFetchTooLarge(Box<SourceContext>),
    #[error(transparent)]
    /// Generic API error.
    Api(Box<ApiContext>),
    #[error(transparent)]
    /// Authentication or authorization API error.
    Auth(Box<ApiContext>),
    #[error(transparent)]
    /// API validation error.
    Validation(Box<ApiContext>),
    #[error(transparent)]
    /// API rate limit error.
    RateLimit(Box<RateLimitContext>),
    #[error(transparent)]
    /// API insufficient credits error.
    InsufficientCredits(Box<CreditsContext>),
    #[error(transparent)]
    /// Terminal job failure error.
    JobFailed(Box<JobContext>),
    #[error(transparent)]
    /// Terminal job cancellation error.
    JobCanceled(Box<JobContext>),
    #[error(transparent)]
    /// SDK-enforced timeout error.
    Timeout(Box<ErrorContext>),
    #[error(transparent)]
    /// Caller-initiated request cancellation error.
    RequestAborted(Box<ErrorContext>),
    #[error(transparent)]
    /// Polling or streaming helper error.
    Stream(Box<StreamContext>),
}

impl ConduitError {
    pub(crate) fn with_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        match self {
            Self::Base(context) => Self::Base(Box::new(context.with_source(source))),
            Self::Initialization(context) => {
                Self::Initialization(Box::new(context.with_source(source)))
            }
            Self::UnsupportedRuntime(context) => {
                Self::UnsupportedRuntime(Box::new(context.with_source(source)))
            }
            Self::WebhookVerification(context) => {
                Self::WebhookVerification(Box::new(context.with_source(source)))
            }
            Self::InvalidSource(context) => {
                Self::InvalidSource(Box::new(context.with_source(source)))
            }
            Self::RemoteFetch(context) => Self::RemoteFetch(Box::new(context.with_source(source))),
            Self::RemoteFetchTimeout(context) => {
                Self::RemoteFetchTimeout(Box::new(context.with_source(source)))
            }
            Self::RemoteFetchTooLarge(context) => {
                Self::RemoteFetchTooLarge(Box::new(context.with_source(source)))
            }
            Self::Timeout(context) => Self::Timeout(Box::new(context.with_source(source))),
            Self::RequestAborted(context) => {
                Self::RequestAborted(Box::new(context.with_source(source)))
            }
            Self::Stream(context) => Self::Stream(Box::new(context.with_source(source))),
            other => other,
        }
    }

    /// Returns the stable error code associated with this failure.
    pub fn code(&self) -> &str {
        match self {
            Self::Base(context)
            | Self::Initialization(context)
            | Self::UnsupportedRuntime(context)
            | Self::WebhookVerification(context)
            | Self::Timeout(context)
            | Self::RequestAborted(context) => &context.code,
            Self::InvalidSource(context)
            | Self::RemoteFetch(context)
            | Self::RemoteFetchTimeout(context)
            | Self::RemoteFetchTooLarge(context) => &context.code,
            Self::Api(context) | Self::Auth(context) | Self::Validation(context) => &context.code,
            Self::RateLimit(context) => &context.code,
            Self::InsufficientCredits(context) => &context.code,
            Self::JobFailed(context) | Self::JobCanceled(context) => &context.code,
            Self::Stream(context) => &context.code,
        }
    }

    /// Returns the API request identifier, when available.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Base(context)
            | Self::Initialization(context)
            | Self::UnsupportedRuntime(context)
            | Self::WebhookVerification(context)
            | Self::Timeout(context)
            | Self::RequestAborted(context) => context.request_id.as_deref(),
            Self::InvalidSource(context)
            | Self::RemoteFetch(context)
            | Self::RemoteFetchTimeout(context)
            | Self::RemoteFetchTooLarge(context) => context.request_id.as_deref(),
            Self::Api(context) | Self::Auth(context) | Self::Validation(context) => {
                context.request_id.as_deref()
            }
            Self::RateLimit(context) => context.request_id.as_deref(),
            Self::InsufficientCredits(context) => context.request_id.as_deref(),
            Self::JobFailed(context) | Self::JobCanceled(context) => context.request_id.as_deref(),
            Self::Stream(context) => context.request_id.as_deref(),
        }
    }

    /// Returns the HTTP status code for API-derived failures.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::InvalidSource(context)
            | Self::RemoteFetch(context)
            | Self::RemoteFetchTimeout(context)
            | Self::RemoteFetchTooLarge(context) => context.status,
            Self::Api(context) | Self::Auth(context) | Self::Validation(context) => {
                Some(context.status)
            }
            Self::RateLimit(context) => Some(context.status),
            Self::InsufficientCredits(context) => Some(context.status),
            _ => None,
        }
    }

    pub(crate) fn base(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Base(Box::new(ErrorContext::new(message, code)))
    }

    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_request")
    }

    pub(crate) fn invalid_response(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_response")
    }

    /// Creates an error used when a webhook payload is malformed after signature verification.
    pub fn invalid_webhook_payload(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_webhook_payload")
    }

    pub(crate) fn initialization(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Initialization(Box::new(ErrorContext::new(message, code)))
    }

    pub(crate) fn webhook(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::WebhookVerification(Box::new(ErrorContext::new(message, code)))
    }

    pub(crate) fn invalid_source(message: impl Into<String>) -> Self {
        Self::InvalidSource(Box::new(SourceContext::new(message, "invalid_source")))
    }

    pub(crate) fn source_too_large(message: impl Into<String>) -> Self {
        Self::InvalidSource(Box::new(SourceContext::new(message, "source_too_large")))
    }

    pub(crate) fn remote_fetch(
        message: impl Into<String>,
        code: impl Into<String>,
        url: Option<String>,
        status: Option<u16>,
    ) -> Self {
        Self::RemoteFetch(Box::new(
            SourceContext::new(message, code).with_remote(url, status),
        ))
    }

    pub(crate) fn remote_fetch_timeout(url: Option<String>, status: Option<u16>) -> Self {
        Self::RemoteFetchTimeout(Box::new(
            SourceContext::new("remote fetch timed out", "remote_fetch_timeout")
                .with_remote(url, status),
        ))
    }

    pub(crate) fn remote_fetch_too_large(url: Option<String>, status: Option<u16>) -> Self {
        Self::RemoteFetchTooLarge(Box::new(
            SourceContext::new("source.url exceeds upload size limit", "source_too_large")
                .with_remote(url, status),
        ))
    }

    pub(crate) fn api(
        status: u16,
        request_id: Option<String>,
        message: impl Into<String>,
        code: impl Into<String>,
        details: Option<Value>,
        retry_after: Option<Duration>,
    ) -> Self {
        let message = message.into();
        let code = code.into();
        match status {
            401 | 403 => Self::Auth(Box::new(ApiContext::new(
                status, request_id, message, code, details,
            ))),
            402 => {
                let (required, available) = read_credit_values(details.as_ref());
                Self::InsufficientCredits(Box::new(CreditsContext {
                    message,
                    code,
                    request_id,
                    status,
                    details: details.map(Box::new),
                    required,
                    available,
                }))
            }
            422 => Self::Validation(Box::new(ApiContext::new(
                status, request_id, message, code, details,
            ))),
            429 => Self::RateLimit(Box::new(RateLimitContext {
                message,
                code,
                request_id,
                status,
                details: details.map(Box::new),
                retry_after,
            })),
            _ => Self::Api(Box::new(ApiContext::new(
                status, request_id, message, code, details,
            ))),
        }
    }

    pub(crate) fn job_failed(
        job_id: impl Into<String>,
        request_id: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::JobFailed(Box::new(JobContext {
            message: message.into(),
            code: code.into(),
            request_id,
            job_id: job_id.into(),
        }))
    }

    pub(crate) fn job_canceled(job_id: impl Into<String>, request_id: Option<String>) -> Self {
        let job_id = job_id.into();
        Self::JobCanceled(Box::new(JobContext {
            message: format!("job {job_id} canceled"),
            code: "job_canceled".into(),
            request_id,
            job_id,
        }))
    }

    pub(crate) fn timeout(message: impl Into<String>, request_id: Option<String>) -> Self {
        Self::Timeout(Box::new(
            ErrorContext::new(message, "timeout").with_request_id(request_id),
        ))
    }

    pub(crate) fn request_aborted(request_id: Option<String>) -> Self {
        Self::RequestAborted(Box::new(
            ErrorContext::new("request aborted by caller", "request_aborted")
                .with_request_id(request_id),
        ))
    }

    pub(crate) fn stream(message: impl Into<String>, job_id: Option<String>) -> Self {
        Self::Stream(Box::new(StreamContext::new(message, job_id)))
    }
}

pub(crate) fn rate_limit_retry_after(error: &ConduitError) -> Option<Duration> {
    let ConduitError::RateLimit(context) = error else {
        return None;
    };
    context.retry_after
}

pub(crate) fn is_retryable_api_error(error: &ConduitError) -> bool {
    let ConduitError::Api(context) = error else {
        return false;
    };
    context.status >= 500
}

pub(crate) fn is_transport_error(error: &ConduitError) -> bool {
    let ConduitError::Base(context) = error else {
        return false;
    };
    context.code == "transport_error"
}

fn read_credit_values(details: Option<&Value>) -> (f64, f64) {
    let Some(Value::Object(map)) = details else {
        return (0.0, 0.0);
    };
    let required = map
        .get("required")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    let available = map
        .get("available")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    (required, available)
}
