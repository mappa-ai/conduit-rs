//! Report workflow types and resource methods.

use crate::common::{path_segment, require_non_empty};
use crate::error::{ConduitError, Result};
use crate::model::{
    Job, JobEvent, JobStage, ReceiptStatus, Report, parse_job_receipt, parse_report,
};
use crate::primitives::{
    ActionOptions, JobsResource, MediaResource, Source, StreamOptions, WaitOptions,
};
use crate::transport::{RequestBody, RequestOptions, Transport};
use futures_util::stream::BoxStream;
use reqwest::Method;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Stable report templates supported by the public API.
pub enum ReportTemplate {
    /// The general-purpose behavioral report template.
    GeneralReport,
    /// A sales-focused report template.
    SalesPlaybook,
}

impl ReportTemplate {
    /// Returns the canonical API identifier for the template.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeneralReport => "general_report",
            Self::SalesPlaybook => "sales_playbook",
        }
    }

    pub(crate) fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "general_report" => Ok(Self::GeneralReport),
            "sales_playbook" => Ok(Self::SalesPlaybook),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported report template"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Behavior used when the requested target cannot be resolved.
pub enum OnMiss {
    /// Fail the request when the target cannot be resolved.
    Error,
    /// Fall back to the dominant speaker when the requested target is unavailable.
    FallbackDominant,
}

impl OnMiss {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::FallbackDominant => "fallback_dominant",
        }
    }
}

#[derive(Debug, Clone)]
/// Webhook destination attached to a create request.
pub struct WebhookEndpoint {
    /// Destination URL that will receive job lifecycle events.
    pub url: String,
    /// Additional headers sent with the webhook request.
    pub headers: HashMap<String, String>,
}

impl WebhookEndpoint {
    /// Creates a webhook endpoint with the given destination URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
        }
    }

    /// Adds a custom header to the webhook destination.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone)]
/// Speaker selection strategy used when creating a report.
pub enum Target {
    /// Select the dominant speaker in the recording.
    Dominant {
        /// Optional fallback behavior if selection fails.
        on_miss: Option<OnMiss>,
    },
    /// Select a speaker from a time range within the recording.
    TimeRange {
        /// Inclusive lower bound, in seconds.
        start_seconds: Option<f64>,
        /// Exclusive upper bound, in seconds.
        end_seconds: Option<f64>,
        /// Optional fallback behavior if selection fails.
        on_miss: Option<OnMiss>,
    },
    /// Select a known entity by stable Conduit entity identifier.
    Entity {
        /// Stable entity identifier.
        entity_id: String,
        /// Optional fallback behavior if selection fails.
        on_miss: Option<OnMiss>,
    },
    /// Select a speaker using a best-effort natural-language hint.
    Hint {
        /// Natural-language selection hint passed to the API unchanged.
        hint: String,
        /// Optional fallback behavior if selection fails.
        on_miss: Option<OnMiss>,
    },
}

impl Target {
    /// Creates a target that selects the dominant speaker.
    pub fn dominant() -> Self {
        Self::Dominant { on_miss: None }
    }

    /// Creates a target that selects a speaker from the provided time range.
    pub fn time_range(start_seconds: Option<f64>, end_seconds: Option<f64>) -> Self {
        Self::TimeRange {
            start_seconds,
            end_seconds,
            on_miss: None,
        }
    }

    /// Creates a target that selects a known entity.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        Self::Entity {
            entity_id: entity_id.into(),
            on_miss: None,
        }
    }

    /// Creates a target that selects a speaker using a natural-language hint.
    pub fn hint(hint: impl Into<String>) -> Self {
        Self::Hint {
            hint: hint.into(),
            on_miss: None,
        }
    }

    /// Sets fallback behavior when the requested target cannot be resolved.
    pub fn on_miss(self, on_miss: OnMiss) -> Self {
        match self {
            Self::Dominant { .. } => Self::Dominant {
                on_miss: Some(on_miss),
            },
            Self::TimeRange {
                start_seconds,
                end_seconds,
                ..
            } => Self::TimeRange {
                start_seconds,
                end_seconds,
                on_miss: Some(on_miss),
            },
            Self::Entity { entity_id, .. } => Self::Entity {
                entity_id,
                on_miss: Some(on_miss),
            },
            Self::Hint { hint, .. } => Self::Hint {
                hint,
                on_miss: Some(on_miss),
            },
        }
    }
}

#[derive(Debug, Clone)]
/// Request payload for [`ReportsResource::create`].
pub struct ReportCreate {
    /// Input media source.
    pub source: Source,
    /// Output template to generate.
    pub template: ReportTemplate,
    /// Optional template-specific JSON object.
    pub template_params: Option<Value>,
    /// Speaker selection strategy.
    pub target: Target,
    /// Optional completion webhook destination.
    pub webhook: Option<WebhookEndpoint>,
    /// Optional idempotency key applied to the create request.
    pub idempotency_key: Option<String>,
    /// Optional request identifier echoed by the API.
    pub request_id: Option<String>,
}

impl ReportCreate {
    /// Creates a new report request.
    pub fn new(source: Source, template: ReportTemplate, target: Target) -> Self {
        Self {
            source,
            template,
            template_params: None,
            target,
            webhook: None,
            idempotency_key: None,
            request_id: None,
        }
    }

    /// Sets template-specific JSON parameters.
    pub fn template_params(mut self, template_params: Value) -> Self {
        self.template_params = Some(template_params);
        self
    }

    /// Attaches a completion webhook to the request.
    pub fn webhook(mut self, webhook: WebhookEndpoint) -> Self {
        self.webhook = Some(webhook);
        self
    }

    /// Sets a caller-supplied idempotency key.
    pub fn idempotency_key(mut self, value: impl Into<String>) -> Self {
        self.idempotency_key = Some(value.into());
        self
    }

    /// Sets a caller-supplied request identifier.
    pub fn request_id(mut self, value: impl Into<String>) -> Self {
        self.request_id = Some(value.into());
        self
    }
}

#[derive(Debug, Clone)]
/// Receipt returned immediately after a report job is accepted.
pub struct ReportReceipt {
    /// Accepted job identifier.
    pub job_id: String,
    /// Initial receipt status reported by the API.
    pub status: ReceiptStatus,
    /// Helper for polling, waiting, and canceling the job.
    pub handle: ReportHandle,
    /// Uploaded media identifier when known.
    pub media_id: Option<String>,
    /// Current job stage, when available.
    pub stage: Option<JobStage>,
    /// Advisory estimated wait time, in seconds.
    pub estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Clone)]
/// Polling and convenience helpers associated with a report receipt.
pub struct ReportHandle {
    job_id: String,
    jobs: JobsResource,
    reports: ReportsResource,
}

#[derive(Debug, Clone)]
/// Reports API resource group.
pub struct ReportsResource {
    transport: Arc<Transport>,
    jobs: JobsResource,
    media: MediaResource,
}

impl ReportsResource {
    pub(crate) fn new(transport: Arc<Transport>, jobs: JobsResource, media: MediaResource) -> Self {
        Self {
            transport,
            jobs,
            media,
        }
    }

    /// Creates a report job.
    ///
    /// This method uploads any non-`media_id` source first, then creates a report job and returns
    /// a receipt without waiting for report generation to finish.
    pub async fn create(&self, request: ReportCreate) -> Result<ReportReceipt> {
        let ReportCreate {
            source,
            template,
            template_params,
            target,
            webhook,
            idempotency_key,
            request_id,
        } = request;
        let media_id = self
            .media
            .resolve_report_source(source, idempotency_key.clone(), request_id.clone())
            .await?;

        let request = ReportCreate {
            source: Source::media_id(&media_id),
            template,
            template_params,
            target,
            webhook,
            idempotency_key,
            request_id,
        };
        let mut body = Map::new();
        body.insert("media".to_string(), json!({ "mediaId": media_id }));
        body.insert("output".to_string(), normalize_report_output(&request)?);
        body.insert("target".to_string(), normalize_target(&request.target)?);
        if let Some(webhook) = request.webhook.as_ref() {
            body.insert("webhook".to_string(), normalize_webhook(webhook)?);
        }

        let response = self
            .transport
            .request(
                Method::POST,
                "/v1/reports/jobs",
                RequestOptions {
                    body: RequestBody::Json(Value::Object(body)),
                    idempotency_key: Some(
                        request
                            .idempotency_key
                            .unwrap_or_else(|| crate::common::random_id("idem")),
                    ),
                    request_id: request.request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        let receipt = parse_job_receipt(&response.body)?;
        Ok(ReportReceipt {
            job_id: receipt.job_id.clone(),
            status: receipt.status,
            media_id: Some(media_id),
            stage: receipt.stage,
            estimated_wait_sec: receipt.estimated_wait_sec,
            handle: ReportHandle {
                job_id: receipt.job_id,
                jobs: self.jobs.clone(),
                reports: self.clone(),
            },
        })
    }

    /// Fetches a completed report by report identifier.
    pub async fn get(&self, report_id: &str) -> Result<Report> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/reports/{}", path_segment(report_id, "report_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_report(&response.body)
    }

    /// Fetches the completed report associated with a job, if one exists yet.
    pub async fn get_by_job(&self, job_id: &str) -> Result<Option<Report>> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/reports/by-job/{}", path_segment(job_id, "job_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        if response.body.is_empty() || String::from_utf8_lossy(&response.body).trim() == "null" {
            return Ok(None);
        }
        Ok(Some(parse_report(&response.body)?))
    }
}

impl ReportHandle {
    /// Polls the job and yields status, stage, and terminal events.
    pub fn stream(&self) -> BoxStream<'static, Result<JobEvent>> {
        self.jobs
            .stream(self.job_id.clone(), StreamOptions::default())
    }

    /// Polls the job using custom stream options.
    pub fn stream_with(&self, options: StreamOptions) -> BoxStream<'static, Result<JobEvent>> {
        self.jobs.stream(self.job_id.clone(), options)
    }

    /// Waits for a terminal job state using default polling settings.
    pub async fn wait(&self) -> Result<Report> {
        self.wait_with(WaitOptions::default()).await
    }

    /// Waits for report completion with a custom timeout.
    pub async fn wait_for(&self, timeout: Duration) -> Result<Report> {
        self.wait_with(WaitOptions::default().timeout(timeout))
            .await
    }

    /// Waits for report completion using custom polling options.
    pub async fn wait_with(&self, options: WaitOptions) -> Result<Report> {
        let job = self.jobs.wait(&self.job_id, options).await?;
        match job.report_id {
            Some(report_id) => self.reports.get(&report_id).await,
            None => Err(ConduitError::invalid_response(format!(
                "job {} succeeded but no reportId was returned",
                self.job_id
            ))),
        }
    }

    /// Requests cancellation for the underlying job.
    pub async fn cancel(&self) -> Result<Job> {
        self.jobs.cancel(&self.job_id).await
    }

    /// Requests cancellation using custom action options.
    pub async fn cancel_with(&self, options: ActionOptions) -> Result<Job> {
        self.jobs.cancel_with(&self.job_id, options).await
    }

    /// Fetches the latest job state.
    pub async fn job(&self) -> Result<Job> {
        self.jobs.get(&self.job_id).await
    }

    /// Fetches the report if the job has already completed successfully.
    pub async fn report(&self) -> Result<Option<Report>> {
        self.reports.get_by_job(&self.job_id).await
    }
}

pub(crate) fn normalize_report_output(request: &ReportCreate) -> Result<Value> {
    if let Some(template_params) = request.template_params.as_ref()
        && !template_params.is_object()
    {
        return Err(ConduitError::invalid_request(
            "output.templateParams must be an object",
        ));
    }

    let mut payload = Map::new();
    payload.insert(
        "template".to_string(),
        Value::String(request.template.as_str().to_string()),
    );
    if let Some(template_params) = request.template_params.as_ref() {
        payload.insert("templateParams".to_string(), template_params.clone());
    }
    Ok(Value::Object(payload))
}

pub(crate) fn normalize_target(target: &Target) -> Result<Value> {
    let mut payload = Map::new();
    match target {
        Target::Dominant { on_miss } => {
            payload.insert(
                "strategy".to_string(),
                Value::String("dominant".to_string()),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        Target::TimeRange {
            start_seconds,
            end_seconds,
            on_miss,
        } => {
            if start_seconds.is_none() && end_seconds.is_none() {
                return Err(ConduitError::invalid_request(
                    "target.timeRange must include startSeconds or endSeconds",
                ));
            }
            if let (Some(start_seconds), Some(end_seconds)) = (start_seconds, end_seconds)
                && start_seconds >= end_seconds
            {
                return Err(ConduitError::invalid_request(
                    "target.timeRange.startSeconds must be less than endSeconds",
                ));
            }
            payload.insert(
                "strategy".to_string(),
                Value::String("timerange".to_string()),
            );
            payload.insert(
                "timeRange".to_string(),
                json!({
                    "startSeconds": start_seconds,
                    "endSeconds": end_seconds,
                }),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        Target::Entity { entity_id, on_miss } => {
            payload.insert(
                "strategy".to_string(),
                Value::String("entity_id".to_string()),
            );
            payload.insert(
                "entityId".to_string(),
                Value::String(require_non_empty(entity_id, "target.entityId")?),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        Target::Hint { hint, on_miss } => {
            payload.insert(
                "strategy".to_string(),
                Value::String("magic_hint".to_string()),
            );
            payload.insert(
                "hint".to_string(),
                Value::String(require_non_empty(hint, "target.hint")?),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
    }
    Ok(Value::Object(payload))
}

pub(crate) fn normalize_webhook(webhook: &WebhookEndpoint) -> Result<Value> {
    let parsed = crate::common::parse_http_url(&webhook.url, "webhook.url")?;
    let mut payload = Map::new();
    payload.insert("url".to_string(), Value::String(parsed.to_string()));
    if !webhook.headers.is_empty() {
        let mut headers = Map::new();
        for (key, value) in &webhook.headers {
            headers.insert(
                key.clone(),
                Value::String(require_non_empty(
                    value,
                    &format!("webhook.headers[{key}]"),
                )?),
            );
        }
        payload.insert("headers".to_string(), Value::Object(headers));
    }
    Ok(Value::Object(payload))
}

fn apply_on_miss(payload: &mut Map<String, Value>, on_miss: Option<OnMiss>) {
    if let Some(on_miss) = on_miss {
        payload.insert(
            "onMiss".to_string(),
            Value::String(on_miss.as_str().to_string()),
        );
    }
}
