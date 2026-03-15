use crate::common::{path_segment, require_non_empty};
use crate::error::{ConduitError, Result};
use crate::model::{Job, JobEvent, Report, parse_job_receipt, parse_report};
use crate::primitives::{JobsResource, MediaResource, Source, StreamOptions, WaitOptions};
use crate::transport::{RequestBody, RequestOptions, Transport};
use futures_util::stream::BoxStream;
use reqwest::Method;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportTemplate {
    GeneralReport,
    SalesPlaybook,
}

impl ReportTemplate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeneralReport => "general_report",
            Self::SalesPlaybook => "sales_playbook",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnMiss {
    Error,
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
pub struct Webhook {
    pub url: String,
    pub headers: HashMap<String, String>,
}

impl Webhook {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReportOutput {
    pub template: ReportTemplate,
    pub template_params: Option<Value>,
}

impl ReportOutput {
    pub fn new(template: ReportTemplate) -> Self {
        Self {
            template,
            template_params: None,
        }
    }

    pub fn with_template_params(mut self, template_params: Value) -> Self {
        self.template_params = Some(template_params);
        self
    }
}

#[derive(Debug, Clone)]
pub enum TargetSelector {
    Dominant {
        on_miss: Option<OnMiss>,
    },
    TimeRange {
        start_seconds: Option<f64>,
        end_seconds: Option<f64>,
        on_miss: Option<OnMiss>,
    },
    EntityId {
        entity_id: String,
        on_miss: Option<OnMiss>,
    },
    MagicHint {
        hint: String,
        on_miss: Option<OnMiss>,
    },
}

impl TargetSelector {
    pub fn dominant() -> Self {
        Self::Dominant { on_miss: None }
    }

    pub fn time_range(start_seconds: Option<f64>, end_seconds: Option<f64>) -> Self {
        Self::TimeRange {
            start_seconds,
            end_seconds,
            on_miss: None,
        }
    }

    pub fn entity_id(entity_id: impl Into<String>) -> Self {
        Self::EntityId {
            entity_id: entity_id.into(),
            on_miss: None,
        }
    }

    pub fn magic_hint(hint: impl Into<String>) -> Self {
        Self::MagicHint {
            hint: hint.into(),
            on_miss: None,
        }
    }

    pub fn with_on_miss(self, on_miss: OnMiss) -> Self {
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
            Self::EntityId { entity_id, .. } => Self::EntityId {
                entity_id,
                on_miss: Some(on_miss),
            },
            Self::MagicHint { hint, .. } => Self::MagicHint {
                hint,
                on_miss: Some(on_miss),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateReportRequest {
    pub source: Source,
    pub output: ReportOutput,
    pub target: TargetSelector,
    pub webhook: Option<Webhook>,
    pub idempotency_key: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReportJobReceipt {
    pub job_id: String,
    pub status: String,
    pub handle: ReportRunHandle,
    pub media_id: Option<String>,
    pub stage: Option<String>,
    pub estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ReportRunHandle {
    job_id: String,
    jobs: JobsResource,
    reports: ReportsResource,
}

#[derive(Debug, Clone)]
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

    pub async fn create(&self, request: CreateReportRequest) -> Result<ReportJobReceipt> {
        let media_id = self
            .media
            .resolve_report_source(
                request.source,
                request.idempotency_key.clone(),
                request.request_id.clone(),
            )
            .await?;

        let mut body = Map::new();
        body.insert("media".to_string(), json!({ "mediaId": media_id }));
        body.insert(
            "output".to_string(),
            normalize_report_output(request.output)?,
        );
        body.insert(
            "target".to_string(),
            normalize_target_selector(&request.target)?,
        );
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
        Ok(ReportJobReceipt {
            job_id: receipt.job_id.clone(),
            status: receipt.status,
            media_id: Some(media_id),
            stage: receipt.stage,
            estimated_wait_sec: receipt.estimated_wait_sec,
            handle: ReportRunHandle {
                job_id: receipt.job_id,
                jobs: self.jobs.clone(),
                reports: self.clone(),
            },
        })
    }

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

impl ReportRunHandle {
    pub fn stream(&self, options: StreamOptions) -> BoxStream<'static, Result<JobEvent>> {
        self.jobs.stream(self.job_id.clone(), options)
    }

    pub async fn wait(&self, options: WaitOptions) -> Result<Report> {
        let job = self.jobs.wait(&self.job_id, options).await?;
        match job.report_id {
            Some(report_id) => self.reports.get(&report_id).await,
            None => Err(ConduitError::invalid_response(format!(
                "job {} succeeded but no reportId was returned",
                self.job_id
            ))),
        }
    }

    pub async fn cancel(
        &self,
        idempotency_key: Option<String>,
        request_id: Option<String>,
    ) -> Result<Job> {
        self.jobs
            .cancel(&self.job_id, idempotency_key, request_id)
            .await
    }

    pub async fn job(&self) -> Result<Job> {
        self.jobs.get(&self.job_id).await
    }

    pub async fn report(&self) -> Result<Option<Report>> {
        self.reports.get_by_job(&self.job_id).await
    }
}

pub(crate) fn normalize_report_output(output: ReportOutput) -> Result<Value> {
    if let Some(template_params) = &output.template_params
        && !template_params.is_object()
    {
        return Err(ConduitError::invalid_request(
            "output.templateParams must be an object",
        ));
    }
    let mut payload = Map::new();
    payload.insert(
        "template".to_string(),
        Value::String(output.template.as_str().to_string()),
    );
    if let Some(template_params) = output.template_params {
        payload.insert("templateParams".to_string(), template_params);
    }
    Ok(Value::Object(payload))
}

pub(crate) fn normalize_target_selector(target: &TargetSelector) -> Result<Value> {
    let mut payload = Map::new();
    match target {
        TargetSelector::Dominant { on_miss } => {
            payload.insert(
                "strategy".to_string(),
                Value::String("dominant".to_string()),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        TargetSelector::TimeRange {
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
                "timerange".to_string(),
                json!({
                    "start_seconds": start_seconds,
                    "end_seconds": end_seconds,
                }),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        TargetSelector::EntityId { entity_id, on_miss } => {
            payload.insert(
                "strategy".to_string(),
                Value::String("entity_id".to_string()),
            );
            payload.insert(
                "entity_id".to_string(),
                Value::String(require_non_empty(entity_id, "target.entityId")?),
            );
            apply_on_miss(&mut payload, *on_miss);
        }
        TargetSelector::MagicHint { hint, on_miss } => {
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

pub(crate) fn normalize_webhook(webhook: &Webhook) -> Result<Value> {
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
            "on_miss".to_string(),
            Value::String(on_miss.as_str().to_string()),
        );
    }
}
