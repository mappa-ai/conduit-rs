use serde_json::Value;
use std::error::Error as StdError;
use std::time::Duration;
use thiserror::Error;

type BoxError = Box<dyn StdError + Send + Sync>;

pub type Result<T> = std::result::Result<T, ConduitError>;

#[derive(Debug, Error)]
pub enum ConduitError {
    #[error("{message}")]
    Base {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    Initialization {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    UnsupportedRuntime {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    WebhookVerification {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    InvalidSource {
        message: String,
        code: String,
        request_id: Option<String>,
        url: Option<String>,
        status: Option<u16>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    RemoteFetch {
        message: String,
        code: String,
        request_id: Option<String>,
        url: Option<String>,
        status: Option<u16>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    RemoteFetchTimeout {
        message: String,
        code: String,
        request_id: Option<String>,
        url: Option<String>,
        status: Option<u16>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    RemoteFetchTooLarge {
        message: String,
        code: String,
        request_id: Option<String>,
        url: Option<String>,
        status: Option<u16>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    Api {
        message: String,
        code: String,
        request_id: Option<String>,
        status: u16,
        details: Option<Value>,
    },
    #[error("{message}")]
    Auth {
        message: String,
        code: String,
        request_id: Option<String>,
        status: u16,
        details: Option<Value>,
    },
    #[error("{message}")]
    Validation {
        message: String,
        code: String,
        request_id: Option<String>,
        status: u16,
        details: Option<Value>,
    },
    #[error("{message}")]
    RateLimit {
        message: String,
        code: String,
        request_id: Option<String>,
        status: u16,
        details: Option<Value>,
        retry_after: Option<Duration>,
    },
    #[error("{message}")]
    InsufficientCredits {
        message: String,
        code: String,
        request_id: Option<String>,
        status: u16,
        details: Option<Value>,
        required: f64,
        available: f64,
    },
    #[error("{message}")]
    JobFailed {
        message: String,
        code: String,
        request_id: Option<String>,
        job_id: String,
    },
    #[error("{message}")]
    JobCanceled {
        message: String,
        code: String,
        request_id: Option<String>,
        job_id: String,
    },
    #[error("{message}")]
    Timeout {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    RequestAborted {
        message: String,
        code: String,
        request_id: Option<String>,
        #[source]
        source: Option<BoxError>,
    },
    #[error("{message}")]
    Stream {
        message: String,
        code: String,
        request_id: Option<String>,
        job_id: Option<String>,
        last_event_id: Option<String>,
        retry_count: usize,
        #[source]
        source: Option<BoxError>,
    },
}

impl ConduitError {
    pub(crate) fn with_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        match self {
            Self::Base {
                message,
                code,
                request_id,
                ..
            } => Self::Base {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::Initialization {
                message,
                code,
                request_id,
                ..
            } => Self::Initialization {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::UnsupportedRuntime {
                message,
                code,
                request_id,
                ..
            } => Self::UnsupportedRuntime {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::WebhookVerification {
                message,
                code,
                request_id,
                ..
            } => Self::WebhookVerification {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::InvalidSource {
                message,
                code,
                request_id,
                url,
                status,
                ..
            } => Self::InvalidSource {
                message,
                code,
                request_id,
                url,
                status,
                source: Some(Box::new(source)),
            },
            Self::RemoteFetch {
                message,
                code,
                request_id,
                url,
                status,
                ..
            } => Self::RemoteFetch {
                message,
                code,
                request_id,
                url,
                status,
                source: Some(Box::new(source)),
            },
            Self::RemoteFetchTimeout {
                message,
                code,
                request_id,
                url,
                status,
                ..
            } => Self::RemoteFetchTimeout {
                message,
                code,
                request_id,
                url,
                status,
                source: Some(Box::new(source)),
            },
            Self::RemoteFetchTooLarge {
                message,
                code,
                request_id,
                url,
                status,
                ..
            } => Self::RemoteFetchTooLarge {
                message,
                code,
                request_id,
                url,
                status,
                source: Some(Box::new(source)),
            },
            Self::Timeout {
                message,
                code,
                request_id,
                ..
            } => Self::Timeout {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::RequestAborted {
                message,
                code,
                request_id,
                ..
            } => Self::RequestAborted {
                message,
                code,
                request_id,
                source: Some(Box::new(source)),
            },
            Self::Stream {
                message,
                code,
                request_id,
                job_id,
                last_event_id,
                retry_count,
                ..
            } => Self::Stream {
                message,
                code,
                request_id,
                job_id,
                last_event_id,
                retry_count,
                source: Some(Box::new(source)),
            },
            other => other,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Base { code, .. }
            | Self::Initialization { code, .. }
            | Self::UnsupportedRuntime { code, .. }
            | Self::WebhookVerification { code, .. }
            | Self::InvalidSource { code, .. }
            | Self::RemoteFetch { code, .. }
            | Self::RemoteFetchTimeout { code, .. }
            | Self::RemoteFetchTooLarge { code, .. }
            | Self::Api { code, .. }
            | Self::Auth { code, .. }
            | Self::Validation { code, .. }
            | Self::RateLimit { code, .. }
            | Self::InsufficientCredits { code, .. }
            | Self::JobFailed { code, .. }
            | Self::JobCanceled { code, .. }
            | Self::Timeout { code, .. }
            | Self::RequestAborted { code, .. }
            | Self::Stream { code, .. } => code,
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Base { request_id, .. }
            | Self::Initialization { request_id, .. }
            | Self::UnsupportedRuntime { request_id, .. }
            | Self::WebhookVerification { request_id, .. }
            | Self::InvalidSource { request_id, .. }
            | Self::RemoteFetch { request_id, .. }
            | Self::RemoteFetchTimeout { request_id, .. }
            | Self::RemoteFetchTooLarge { request_id, .. }
            | Self::Api { request_id, .. }
            | Self::Auth { request_id, .. }
            | Self::Validation { request_id, .. }
            | Self::RateLimit { request_id, .. }
            | Self::InsufficientCredits { request_id, .. }
            | Self::JobFailed { request_id, .. }
            | Self::JobCanceled { request_id, .. }
            | Self::Timeout { request_id, .. }
            | Self::RequestAborted { request_id, .. }
            | Self::Stream { request_id, .. } => request_id.as_deref(),
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Self::InvalidSource { status, .. }
            | Self::RemoteFetch { status, .. }
            | Self::RemoteFetchTimeout { status, .. }
            | Self::RemoteFetchTooLarge { status, .. } => *status,
            Self::Api { status, .. }
            | Self::Auth { status, .. }
            | Self::Validation { status, .. }
            | Self::RateLimit { status, .. }
            | Self::InsufficientCredits { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub(crate) fn base(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Base {
            message: message.into(),
            code: code.into(),
            request_id: None,
            source: None,
        }
    }

    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_request")
    }

    pub(crate) fn invalid_response(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_response")
    }

    pub fn invalid_webhook_payload(message: impl Into<String>) -> Self {
        Self::base(message, "invalid_webhook_payload")
    }

    pub(crate) fn initialization(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Initialization {
            message: message.into(),
            code: code.into(),
            request_id: None,
            source: None,
        }
    }

    pub(crate) fn webhook(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::WebhookVerification {
            message: message.into(),
            code: code.into(),
            request_id: None,
            source: None,
        }
    }

    pub(crate) fn invalid_source(message: impl Into<String>) -> Self {
        Self::InvalidSource {
            message: message.into(),
            code: "invalid_source".into(),
            request_id: None,
            url: None,
            status: None,
            source: None,
        }
    }

    pub(crate) fn source_too_large(message: impl Into<String>) -> Self {
        Self::InvalidSource {
            message: message.into(),
            code: "source_too_large".into(),
            request_id: None,
            url: None,
            status: None,
            source: None,
        }
    }

    pub(crate) fn remote_fetch(
        message: impl Into<String>,
        code: impl Into<String>,
        url: Option<String>,
        status: Option<u16>,
    ) -> Self {
        Self::RemoteFetch {
            message: message.into(),
            code: code.into(),
            request_id: None,
            url,
            status,
            source: None,
        }
    }

    pub(crate) fn remote_fetch_timeout(url: Option<String>, status: Option<u16>) -> Self {
        Self::RemoteFetchTimeout {
            message: "remote fetch timed out".into(),
            code: "remote_fetch_timeout".into(),
            request_id: None,
            url,
            status,
            source: None,
        }
    }

    pub(crate) fn remote_fetch_too_large(url: Option<String>, status: Option<u16>) -> Self {
        Self::RemoteFetchTooLarge {
            message: "source.url exceeds upload size limit".into(),
            code: "source_too_large".into(),
            request_id: None,
            url,
            status,
            source: None,
        }
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
            401 | 403 => Self::Auth {
                message,
                code,
                request_id,
                status,
                details,
            },
            402 => {
                let (required, available) = read_credit_values(details.as_ref());
                Self::InsufficientCredits {
                    message,
                    code,
                    request_id,
                    status,
                    details,
                    required,
                    available,
                }
            }
            422 => Self::Validation {
                message,
                code,
                request_id,
                status,
                details,
            },
            429 => Self::RateLimit {
                message,
                code,
                request_id,
                status,
                details,
                retry_after,
            },
            _ => Self::Api {
                message,
                code,
                request_id,
                status,
                details,
            },
        }
    }

    pub(crate) fn job_failed(
        job_id: impl Into<String>,
        request_id: Option<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::JobFailed {
            message: message.into(),
            code: code.into(),
            request_id,
            job_id: job_id.into(),
        }
    }

    pub(crate) fn job_canceled(job_id: impl Into<String>, request_id: Option<String>) -> Self {
        let job_id = job_id.into();
        Self::JobCanceled {
            message: format!("job {job_id} canceled"),
            code: "job_canceled".into(),
            request_id,
            job_id,
        }
    }

    pub(crate) fn timeout(message: impl Into<String>, request_id: Option<String>) -> Self {
        Self::Timeout {
            message: message.into(),
            code: "timeout".into(),
            request_id,
            source: None,
        }
    }

    pub(crate) fn request_aborted(request_id: Option<String>) -> Self {
        Self::RequestAborted {
            message: "request aborted by caller".into(),
            code: "request_aborted".into(),
            request_id,
            source: None,
        }
    }

    pub(crate) fn stream(message: impl Into<String>, job_id: Option<String>) -> Self {
        Self::Stream {
            message: message.into(),
            code: "stream_error".into(),
            request_id: None,
            job_id,
            last_event_id: None,
            retry_count: 0,
            source: None,
        }
    }
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
