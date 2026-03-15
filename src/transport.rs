use crate::common::random_id;
use crate::error::{
    ConduitError, Result, is_retryable_api_error, is_transport_error, rate_limit_retry_after,
};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use reqwest::{Method, Url};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct MultipartFile {
    pub file_name: String,
    pub content_type: Option<String>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) enum RequestBody {
    Empty,
    Json(Value),
    Multipart {
        fields: Vec<(String, String)>,
        file: MultipartFile,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct RequestOptions {
    pub body: RequestBody,
    pub query: Vec<(String, String)>,
    pub idempotency_key: Option<String>,
    pub request_id: Option<String>,
    pub retryable: bool,
    pub timeout: Option<Duration>,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            body: RequestBody::Empty,
            query: Vec::new(),
            idempotency_key: None,
            request_id: None,
            retryable: false,
            timeout: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TransportResponse {
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct Transport {
    api_key: String,
    base_url: Url,
    client: reqwest::Client,
    timeout: Duration,
    max_retries: usize,
    user_agent: Option<String>,
}

impl Transport {
    pub(crate) fn new(
        api_key: String,
        base_url: Url,
        timeout: Duration,
        max_retries: usize,
        user_agent: Option<String>,
    ) -> Self {
        let client = reqwest::Client::builder().build().expect("reqwest client");
        Self {
            api_key,
            base_url,
            client,
            timeout,
            max_retries,
            user_agent,
        }
    }

    pub(crate) async fn request(
        &self,
        method: Method,
        path: &str,
        options: RequestOptions,
    ) -> Result<TransportResponse> {
        let request_id = options
            .request_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| random_id("req"));
        let attempts = if options.retryable {
            self.max_retries + 1
        } else {
            1
        };

        for attempt in 1..=attempts {
            match self
                .request_once(method.clone(), path, &request_id, &options)
                .await
            {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == attempts || !should_retry(&error) {
                        return Err(error);
                    }
                    tokio::time::sleep(retry_delay(&error, attempt)).await;
                }
            }
        }

        Err(ConduitError::base(
            "unexpected transport exit",
            "transport_error",
        ))
    }

    async fn request_once(
        &self,
        method: Method,
        path: &str,
        request_id: &str,
        options: &RequestOptions,
    ) -> Result<TransportResponse> {
        let mut url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| {
                ConduitError::base("failed to build request URL", "transport_error")
                    .with_source(error)
            })?;
        if !options.query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in &options.query {
                pairs.append_pair(key, value);
            }
        }

        let mut request = self
            .client
            .request(method, url)
            .timeout(options.timeout.unwrap_or(self.timeout))
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .header("Mappa-Api-Key", &self.api_key)
            .header("X-Request-Id", request_id);

        if let Some(user_agent) = &self.user_agent {
            request = request.header("User-Agent", user_agent);
        }
        if let Some(idempotency_key) = &options.idempotency_key {
            request = request.header("Idempotency-Key", idempotency_key);
        }

        match &options.body {
            RequestBody::Empty => {}
            RequestBody::Json(body) => {
                request = request.json(body);
            }
            RequestBody::Multipart { fields, file } => {
                let mut form = reqwest::multipart::Form::new();
                for (key, value) in fields {
                    form = form.text(key.clone(), value.clone());
                }
                let mut part = reqwest::multipart::Part::bytes(file.payload.clone())
                    .file_name(file.file_name.clone());
                if let Some(content_type) = &file.content_type {
                    part = part.mime_str(content_type).map_err(|error| {
                        ConduitError::invalid_request("invalid upload content type")
                            .with_source(error)
                    })?;
                }
                form = form.part("file", part);
                request = request.multipart(form);
            }
        }

        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                return ConduitError::timeout(
                    format!(
                        "request timed out after {}ms",
                        options.timeout.unwrap_or(self.timeout).as_millis()
                    ),
                    Some(request_id.to_string()),
                )
                .with_source(error);
            }
            if error.is_request() {
                return ConduitError::request_aborted(Some(request_id.to_string()))
                    .with_source(error);
            }
            ConduitError::base("request failed", "transport_error").with_source(error)
        })?;

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let server_request_id = headers
            .get("X-Request-Id")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(request_id)
            .to_string();
        let body = response.bytes().await.map_err(|error| {
            ConduitError::base("failed to read response", "transport_error").with_source(error)
        })?;
        if (200..300).contains(&status) {
            return Ok(TransportResponse {
                body: body.to_vec(),
            });
        }

        Err(parse_api_error(
            status,
            &body,
            &headers,
            Some(server_request_id),
        ))
    }
}

fn parse_api_error(
    status: u16,
    body: &[u8],
    headers: &HeaderMap,
    request_id: Option<String>,
) -> ConduitError {
    let retry_after = headers
        .get("Retry-After")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after);

    let mut message = "Request failed".to_string();
    let mut code = "api_error".to_string();
    let mut details = None;

    if let Ok(payload) = serde_json::from_slice::<Value>(body) {
        if let Some(error) = payload.get("error").and_then(Value::as_object) {
            if let Some(value) = error.get("message").and_then(Value::as_str) {
                message = value.to_string();
            }
            if let Some(value) = error.get("code").and_then(Value::as_str) {
                code = value.to_string();
            }
            details = error.get("details").cloned();
        } else if let Some(object) = payload.as_object() {
            if let Some(value) = object.get("message").and_then(Value::as_str) {
                message = value.to_string();
            }
            if let Some(value) = object.get("code").and_then(Value::as_str) {
                code = value.to_string();
            }
            details = Some(payload);
        }
    } else if let Ok(text) = std::str::from_utf8(body) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            message = trimmed.to_string();
        }
    }

    ConduitError::api(status, request_id, message, code, details, retry_after)
}

fn should_retry(error: &ConduitError) -> bool {
    match error {
        ConduitError::RateLimit(_) | ConduitError::Timeout(_) => true,
        _ if is_retryable_api_error(error) => true,
        _ if is_transport_error(error) => true,
        _ => false,
    }
}

fn retry_delay(error: &ConduitError, attempt: usize) -> Duration {
    if let Some(retry_after) = rate_limit_retry_after(error) {
        return retry_after;
    }
    let seconds = 2u64.pow(attempt.min(3) as u32);
    Duration::from_millis((seconds * 500).min(4_000))
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}
