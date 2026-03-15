use crate::error::{ConduitError, Result};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Url;
use std::path::Path;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

pub(crate) fn require_non_empty(value: &str, name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConduitError::invalid_request(format!(
            "{name} must be a non-empty string"
        )));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn response_string(value: &str, name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConduitError::invalid_response(format!(
            "invalid {name}: expected string"
        )));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn path_segment(value: &str, name: &str) -> Result<String> {
    Ok(utf8_percent_encode(&require_non_empty(value, name)?, NON_ALPHANUMERIC).to_string())
}

pub(crate) fn random_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

pub(crate) fn parse_http_url(value: &str, name: &str) -> Result<Url> {
    let normalized = require_non_empty(value, name)?;
    let parsed = Url::parse(&normalized).map_err(|error| {
        ConduitError::invalid_source(format!("{name} must be an http or https URL"))
            .with_source(error)
    })?;
    if (parsed.scheme() != "http" && parsed.scheme() != "https") || parsed.host_str().is_none() {
        return Err(ConduitError::invalid_source(format!(
            "{name} must be an http or https URL"
        )));
    }
    Ok(parsed)
}

pub(crate) fn parse_base_url(value: &str) -> Result<Url> {
    let normalized = value.trim();
    let parsed = Url::parse(normalized).map_err(|error| {
        ConduitError::initialization("base_url must be a valid URL", "config_error")
            .with_source(error)
    })?;
    if (parsed.scheme() != "http" && parsed.scheme() != "https") || parsed.host_str().is_none() {
        return Err(ConduitError::initialization(
            "base_url must be a valid URL",
            "config_error",
        ));
    }
    Ok(parsed)
}

pub(crate) fn content_type_from_name(name: &str) -> Option<String> {
    mime_guess::from_path(name)
        .first_raw()
        .map(ToString::to_string)
}

pub(crate) fn strip_content_type(value: Option<&str>) -> Option<String> {
    value
        .map(|item| {
            item.split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .filter(|item| !item.is_empty())
}

pub(crate) fn resolve_label(label: Option<&str>, file_name: &str, fallback: &str) -> String {
    if let Some(label) = label {
        let normalized = normalize_label(label);
        if !normalized.is_empty() {
            return normalized;
        }
    }
    let normalized = normalize_label(file_name);
    if !normalized.is_empty() {
        return normalized;
    }
    fallback.to_string()
}

pub(crate) fn normalize_label(value: &str) -> String {
    let mut current = value.trim().to_string();
    loop {
        let Some((base, suffix)) = current.rsplit_once('.') else {
            break;
        };
        if base.trim().is_empty() || suffix.trim().is_empty() {
            break;
        }
        current = base.trim().to_string();
    }
    current.trim().to_string()
}

pub(crate) fn file_name_from_url(url: &Url) -> String {
    Path::new(url.path())
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| "remote.bin".to_string())
}

pub(crate) fn is_terminal_status(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "canceled")
}

pub(crate) fn parse_iso8601(value: &str, name: &str) -> Result<()> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        ConduitError::invalid_webhook_payload(format!("{name} must be an ISO8601 string"))
            .with_source(error)
    })?;
    Ok(())
}
