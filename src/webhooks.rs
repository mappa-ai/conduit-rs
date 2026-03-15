//! Webhook verification and typed event parsing.

use crate::error::{ConduitError, Result};
use crate::model::JobStatus;
use hmac::{Hmac, Mac};
use http::HeaderMap;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TOLERANCE: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
/// Structured failure payload carried by `*.failed` webhook events.
pub struct WebhookFailure {
    /// Stable error code.
    pub code: String,
    /// Human-readable failure message.
    pub message: String,
}

#[derive(Debug, Clone)]
/// Parsed payload for a `report.completed` webhook event.
pub struct ReportCompletedEvent {
    /// Stable webhook event identifier.
    pub id: String,
    /// Event creation time from the webhook envelope.
    pub created_at: OffsetDateTime,
    /// Duplicate event timestamp retained for compatibility with webhook processors.
    pub timestamp: OffsetDateTime,
    /// Completed job identifier.
    pub job_id: String,
    /// Completed report identifier.
    pub report_id: String,
    /// Terminal job status.
    pub status: JobStatus,
}

#[derive(Debug, Clone)]
/// Parsed payload for a `report.failed` webhook event.
pub struct ReportFailedEvent {
    /// Stable webhook event identifier.
    pub id: String,
    /// Event creation time from the webhook envelope.
    pub created_at: OffsetDateTime,
    /// Duplicate event timestamp retained for compatibility with webhook processors.
    pub timestamp: OffsetDateTime,
    /// Failed job identifier.
    pub job_id: String,
    /// Terminal job status.
    pub status: JobStatus,
    /// Structured failure payload.
    pub error: WebhookFailure,
}

#[derive(Debug, Clone)]
/// Parsed payload for a `matching.completed` webhook event.
pub struct MatchingCompletedEvent {
    /// Stable webhook event identifier.
    pub id: String,
    /// Event creation time from the webhook envelope.
    pub created_at: OffsetDateTime,
    /// Duplicate event timestamp retained for compatibility with webhook processors.
    pub timestamp: OffsetDateTime,
    /// Completed job identifier.
    pub job_id: String,
    /// Completed matching result identifier.
    pub matching_id: String,
    /// Terminal job status.
    pub status: JobStatus,
}

#[derive(Debug, Clone)]
/// Parsed payload for a `matching.failed` webhook event.
pub struct MatchingFailedEvent {
    /// Stable webhook event identifier.
    pub id: String,
    /// Event creation time from the webhook envelope.
    pub created_at: OffsetDateTime,
    /// Duplicate event timestamp retained for compatibility with webhook processors.
    pub timestamp: OffsetDateTime,
    /// Failed job identifier.
    pub job_id: String,
    /// Terminal job status.
    pub status: JobStatus,
    /// Structured failure payload.
    pub error: WebhookFailure,
}

#[derive(Debug, Clone)]
/// Parsed payload for an unknown future webhook event type.
pub struct UnknownWebhookEvent {
    /// Stable webhook event identifier.
    pub id: String,
    /// Event type string preserved from the webhook envelope.
    pub event_type: String,
    /// Event creation time from the webhook envelope.
    pub created_at: OffsetDateTime,
    /// Duplicate event timestamp retained for compatibility with webhook processors.
    pub timestamp: OffsetDateTime,
    /// Opaque event payload preserved as JSON.
    pub data: Value,
}

#[derive(Debug, Clone)]
/// Typed webhook event returned by [`WebhooksResource::parse_event`].
pub enum WebhookEvent {
    /// `report.completed`
    ReportCompleted(ReportCompletedEvent),
    /// `report.failed`
    ReportFailed(ReportFailedEvent),
    /// `matching.completed`
    MatchingCompleted(MatchingCompletedEvent),
    /// `matching.failed`
    MatchingFailed(MatchingFailedEvent),
    /// Any future event type not yet modeled by this SDK version.
    Unknown(UnknownWebhookEvent),
}

#[derive(Debug, Clone, Default)]
/// Webhook signature verification and event parsing helpers.
pub struct WebhooksResource;

impl WebhooksResource {
    /// Verifies a webhook signature using the default five-minute tolerance window.
    ///
    /// Pass the exact raw request body. Parsed JSON bodies will fail verification.
    pub fn verify_signature(
        &self,
        payload: &[u8],
        headers: &HeaderMap,
        secret: &str,
    ) -> Result<()> {
        self.verify_signature_with_tolerance(payload, headers, secret, DEFAULT_TOLERANCE)
    }

    /// Verifies a webhook signature using a custom tolerance window.
    pub fn verify_signature_with_tolerance(
        &self,
        payload: &[u8],
        headers: &HeaderMap,
        secret: &str,
        tolerance: Duration,
    ) -> Result<()> {
        let tolerance = if tolerance.is_zero() {
            DEFAULT_TOLERANCE
        } else {
            tolerance
        };
        let mut signatures = headers.get_all("conduit-signature").iter();
        let Some(raw_header) = signatures.next() else {
            return Err(ConduitError::webhook(
                "missing conduit-signature header",
                "webhook_signature_missing",
            ));
        };
        if signatures.next().is_some() {
            return Err(ConduitError::webhook(
                "duplicate conduit-signature header",
                "webhook_signature_invalid",
            ));
        }
        let raw_header = raw_header.to_str().map_err(|error| {
            ConduitError::webhook(
                "malformed conduit-signature header",
                "webhook_signature_invalid",
            )
            .with_source(error)
        })?;
        let (timestamp, signature) = parse_signature_header(raw_header)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                ConduitError::webhook("invalid system clock", "webhook_signature_invalid")
                    .with_source(error)
            })?
            .as_secs() as i64;
        if (now - timestamp).unsigned_abs() > tolerance.as_secs() {
            return Err(ConduitError::webhook(
                "signature timestamp outside tolerance",
                "webhook_signature_stale",
            ));
        }

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|error| {
            ConduitError::webhook("invalid webhook secret", "webhook_signature_invalid")
                .with_source(error)
        })?;
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        mac.verify_slice(&signature)
            .map_err(|_| ConduitError::webhook("invalid signature", "webhook_signature_invalid"))
    }

    /// Parses a verified webhook payload into a typed [`WebhookEvent`].
    ///
    /// Unknown future event types are preserved as [`WebhookEvent::Unknown`].
    pub fn parse_event(&self, payload: &[u8]) -> Result<WebhookEvent> {
        let envelope: WebhookEnvelope = serde_json::from_slice(payload).map_err(|error| {
            ConduitError::invalid_webhook_payload("invalid webhook payload: invalid JSON")
                .with_source(error)
        })?;

        let id = non_empty(&envelope.id, "webhook.id")?;
        let created_at = webhook_datetime(&envelope.created_at, "webhook.createdAt")?;
        let timestamp = webhook_datetime(&envelope.timestamp, "webhook.timestamp")?;

        match envelope.kind.as_str() {
            "report.completed" => {
                let payload = parse_completed_payload(envelope.data, "reportId")?;
                Ok(WebhookEvent::ReportCompleted(ReportCompletedEvent {
                    id,
                    created_at,
                    timestamp,
                    job_id: payload.job_id,
                    report_id: payload.resource_id,
                    status: payload.status,
                }))
            }
            "matching.completed" => {
                let payload = parse_completed_payload(envelope.data, "matchingId")?;
                Ok(WebhookEvent::MatchingCompleted(MatchingCompletedEvent {
                    id,
                    created_at,
                    timestamp,
                    job_id: payload.job_id,
                    matching_id: payload.resource_id,
                    status: payload.status,
                }))
            }
            "report.failed" => {
                let payload = parse_failed_payload(envelope.data)?;
                Ok(WebhookEvent::ReportFailed(ReportFailedEvent {
                    id,
                    created_at,
                    timestamp,
                    job_id: payload.job_id,
                    status: payload.status,
                    error: payload.error,
                }))
            }
            "matching.failed" => {
                let payload = parse_failed_payload(envelope.data)?;
                Ok(WebhookEvent::MatchingFailed(MatchingFailedEvent {
                    id,
                    created_at,
                    timestamp,
                    job_id: payload.job_id,
                    status: payload.status,
                    error: payload.error,
                }))
            }
            _ => Ok(WebhookEvent::Unknown(UnknownWebhookEvent {
                id,
                event_type: non_empty(&envelope.kind, "webhook.type")?,
                created_at,
                timestamp,
                data: envelope.data,
            })),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WebhookEnvelope {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    timestamp: String,
    data: Value,
}

struct CompletedPayload {
    job_id: String,
    resource_id: String,
    status: JobStatus,
}

struct FailedPayload {
    job_id: String,
    status: JobStatus,
    error: WebhookFailure,
}

fn parse_signature_header(value: &str) -> Result<(i64, Vec<u8>)> {
    let mut timestamp = None;
    let mut signature = None;
    for part in value.split(',') {
        let Some((key, raw_value)) = part.split_once('=') else {
            return Err(ConduitError::webhook(
                "malformed conduit-signature header",
                "webhook_signature_invalid",
            ));
        };
        let key = key.trim();
        let raw_value = raw_value.trim();
        if key.is_empty() || raw_value.is_empty() || (key != "t" && key != "v1") {
            return Err(ConduitError::webhook(
                "malformed conduit-signature header",
                "webhook_signature_invalid",
            ));
        }
        match key {
            "t" if timestamp.is_none() => {
                timestamp = Some(raw_value.parse::<i64>().map_err(|error| {
                    ConduitError::webhook(
                        "invalid signature timestamp",
                        "webhook_signature_invalid",
                    )
                    .with_source(error)
                })?)
            }
            "v1" if signature.is_none() => {
                signature = Some(hex::decode(raw_value).map_err(|error| {
                    ConduitError::webhook(
                        "malformed conduit-signature header",
                        "webhook_signature_invalid",
                    )
                    .with_source(error)
                })?)
            }
            _ => {
                return Err(ConduitError::webhook(
                    "malformed conduit-signature header",
                    "webhook_signature_invalid",
                ));
            }
        }
    }
    match (timestamp, signature) {
        (Some(timestamp), Some(signature)) => Ok((timestamp, signature)),
        _ => Err(ConduitError::webhook(
            "malformed conduit-signature header",
            "webhook_signature_invalid",
        )),
    }
}

fn parse_completed_payload(data: Value, resource_key: &str) -> Result<CompletedPayload> {
    let object = data.as_object().ok_or_else(|| {
        ConduitError::invalid_webhook_payload("invalid webhook payload: data must be an object")
    })?;
    let job_id = required_value(object.get("jobId"), "webhook.data.jobId")?;
    let resource_id = required_value(
        object.get(resource_key),
        &format!("webhook.data.{resource_key}"),
    )?;
    let status = parse_webhook_status(object.get("status"), JobStatus::Succeeded, "succeeded")?;
    Ok(CompletedPayload {
        job_id,
        resource_id,
        status,
    })
}

fn parse_failed_payload(data: Value) -> Result<FailedPayload> {
    let object = data.as_object().ok_or_else(|| {
        ConduitError::invalid_webhook_payload("invalid webhook payload: data must be an object")
    })?;
    let error = object
        .get("error")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            ConduitError::invalid_webhook_payload(
                "invalid webhook payload: error must be an object",
            )
        })?;
    Ok(FailedPayload {
        job_id: required_value(object.get("jobId"), "webhook.data.jobId")?,
        status: parse_webhook_status(object.get("status"), JobStatus::Failed, "failed")?,
        error: WebhookFailure {
            code: required_value(error.get("code"), "webhook.data.error.code")?,
            message: required_value(error.get("message"), "webhook.data.error.message")?,
        },
    })
}

fn parse_webhook_status(
    value: Option<&Value>,
    expected: JobStatus,
    expected_name: &str,
) -> Result<JobStatus> {
    let status = match value.and_then(Value::as_str) {
        Some("succeeded") => JobStatus::Succeeded,
        Some("failed") => JobStatus::Failed,
        Some(other) => {
            return Err(ConduitError::invalid_webhook_payload(format!(
                "invalid webhook payload: unsupported status {other}"
            )));
        }
        None => {
            return Err(ConduitError::invalid_webhook_payload(format!(
                "invalid webhook payload: status must be {expected_name}"
            )));
        }
    };
    if status != expected {
        return Err(ConduitError::invalid_webhook_payload(format!(
            "invalid webhook payload: status must be {expected_name}"
        )));
    }
    Ok(status)
}

fn required_value(value: Option<&Value>, name: &str) -> Result<String> {
    let value = value.and_then(Value::as_str).unwrap_or_default();
    non_empty(value, name)
}

fn non_empty(value: &str, name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConduitError::invalid_webhook_payload(format!(
            "{name} must be a non-empty string"
        )));
    }
    Ok(trimmed.to_string())
}

fn webhook_datetime(value: &str, name: &str) -> Result<OffsetDateTime> {
    let value = non_empty(value, name)?;
    OffsetDateTime::parse(&value, &Rfc3339).map_err(|error| {
        ConduitError::invalid_webhook_payload(format!("{name} must be an ISO8601 string"))
            .with_source(error)
    })
}
