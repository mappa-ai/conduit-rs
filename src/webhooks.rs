use crate::error::{ConduitError, Result};
use crate::model::{WebhookEvent, parse_webhook_event};
use hex::encode as hex_encode;
use hmac::{Hmac, Mac};
use http::HeaderMap;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Default)]
pub struct WebhooksResource;

impl WebhooksResource {
    pub fn verify_signature(
        &self,
        payload: &[u8],
        headers: &HeaderMap,
        secret: &str,
        tolerance: Duration,
    ) -> Result<()> {
        let tolerance = if tolerance.is_zero() {
            Duration::from_secs(300)
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
        let expected = hex_encode(mac.finalize().into_bytes());
        if expected.as_bytes() != signature.as_bytes() {
            return Err(ConduitError::webhook(
                "invalid signature",
                "webhook_signature_invalid",
            ));
        }
        Ok(())
    }

    pub fn parse_event(&self, payload: &[u8]) -> Result<WebhookEvent> {
        let event = parse_webhook_event(payload)?;
        match event.r#type.as_str() {
            "report.completed" => validate_completed_data(&event.data, "reportId")?,
            "matching.completed" => validate_completed_data(&event.data, "matchingId")?,
            "report.failed" | "matching.failed" => validate_failed_data(&event.data)?,
            _ => {}
        }
        Ok(event)
    }
}

fn parse_signature_header(value: &str) -> Result<(i64, String)> {
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
            "v1" if signature.is_none() => signature = Some(raw_value.to_string()),
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

fn validate_completed_data(data: &serde_json::Value, resource_key: &str) -> Result<()> {
    let object = data.as_object().ok_or_else(|| {
        ConduitError::invalid_webhook_payload("invalid webhook payload: data must be an object")
    })?;
    let job_id = object
        .get("jobId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if job_id.trim().is_empty() {
        return Err(ConduitError::invalid_webhook_payload(
            "webhook.data.jobId must be a non-empty string",
        ));
    }
    let resource_id = object
        .get(resource_key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if resource_id.trim().is_empty() {
        return Err(ConduitError::invalid_webhook_payload(format!(
            "webhook.data.{resource_key} must be a non-empty string"
        )));
    }
    if object.get("status").and_then(serde_json::Value::as_str) != Some("succeeded") {
        return Err(ConduitError::invalid_webhook_payload(
            "invalid webhook payload: status must be succeeded",
        ));
    }
    Ok(())
}

fn validate_failed_data(data: &serde_json::Value) -> Result<()> {
    let object = data.as_object().ok_or_else(|| {
        ConduitError::invalid_webhook_payload("invalid webhook payload: data must be an object")
    })?;
    let job_id = object
        .get("jobId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if job_id.trim().is_empty() {
        return Err(ConduitError::invalid_webhook_payload(
            "webhook.data.jobId must be a non-empty string",
        ));
    }
    if object.get("status").and_then(serde_json::Value::as_str) != Some("failed") {
        return Err(ConduitError::invalid_webhook_payload(
            "invalid webhook payload: status must be failed",
        ));
    }
    let error = object
        .get("error")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            ConduitError::invalid_webhook_payload(
                "invalid webhook payload: error must be an object",
            )
        })?;
    if error
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(ConduitError::invalid_webhook_payload(
            "webhook.data.error.code must be a non-empty string",
        ));
    }
    if error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(ConduitError::invalid_webhook_payload(
            "webhook.data.error.message must be a non-empty string",
        ));
    }
    Ok(())
}
