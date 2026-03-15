//! Matching workflow types and resource methods.

use crate::common::{path_segment, require_non_empty};
use crate::error::{ConduitError, Result};
use crate::model::{
    Job, JobEvent, JobStage, Matching, ReceiptStatus, parse_job_receipt, parse_matching,
};
use crate::primitives::{ActionOptions, JobsResource, StreamOptions, WaitOptions};
use crate::reports::{Target, WebhookEndpoint, normalize_target, normalize_webhook};
use crate::transport::{RequestBody, RequestOptions, Transport};
use futures_util::stream::BoxStream;
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Stable matching contexts supported by the public API.
pub enum MatchingContext {
    /// Hiring team fit analysis.
    HiringTeamFit,
}

impl MatchingContext {
    /// Returns the canonical API identifier for the context.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HiringTeamFit => "hiring_team_fit",
        }
    }

    pub(crate) fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "hiring_team_fit" => Ok(Self::HiringTeamFit),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported matching context"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
/// Reference to a matching subject.
pub enum SubjectRef {
    /// Reference an existing stable entity.
    Entity {
        /// Stable entity identifier.
        entity_id: String,
    },
    /// Reference a selected speaker inside an uploaded media file.
    Media {
        /// Uploaded media identifier.
        media_id: String,
        /// Speaker selection strategy applied to the media.
        selector: Target,
    },
}

impl SubjectRef {
    /// Creates a subject reference for a stable entity.
    pub fn entity(entity_id: impl Into<String>) -> Self {
        Self::Entity {
            entity_id: entity_id.into(),
        }
    }

    /// Creates a subject reference for a media target selected by [`Target`].
    pub fn media(media_id: impl Into<String>, selector: Target) -> Self {
        Self::Media {
            media_id: media_id.into(),
            selector,
        }
    }
}

#[derive(Debug, Clone)]
/// Request payload for [`MatchingResource::create`].
pub struct MatchingCreate {
    /// Matching context to evaluate.
    pub context: MatchingContext,
    /// Subject being evaluated.
    pub target: SubjectRef,
    /// Comparison group. Must contain at least one subject.
    pub group: Vec<SubjectRef>,
    /// Optional completion webhook destination.
    pub webhook: Option<WebhookEndpoint>,
    /// Optional idempotency key applied to the create request.
    pub idempotency_key: Option<String>,
    /// Optional request identifier echoed by the API.
    pub request_id: Option<String>,
}

impl MatchingCreate {
    /// Creates a new matching request.
    pub fn new(context: MatchingContext, target: SubjectRef, group: Vec<SubjectRef>) -> Self {
        Self {
            context,
            target,
            group,
            webhook: None,
            idempotency_key: None,
            request_id: None,
        }
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
/// Receipt returned immediately after a matching job is accepted.
pub struct MatchingReceipt {
    /// Accepted job identifier.
    pub job_id: String,
    /// Initial receipt status reported by the API.
    pub status: ReceiptStatus,
    /// Helper for polling, waiting, and canceling the job.
    pub handle: MatchingHandle,
    /// Current job stage, when available.
    pub stage: Option<JobStage>,
    /// Advisory estimated wait time, in seconds.
    pub estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Clone)]
/// Polling and convenience helpers associated with a matching receipt.
pub struct MatchingHandle {
    job_id: String,
    jobs: JobsResource,
    matching: MatchingResource,
}

#[derive(Debug, Clone)]
/// Matching API resource group.
pub struct MatchingResource {
    transport: Arc<Transport>,
    jobs: JobsResource,
}

impl MatchingResource {
    pub(crate) fn new(transport: Arc<Transport>, jobs: JobsResource) -> Self {
        Self { transport, jobs }
    }

    /// Creates a matching job.
    ///
    /// This method validates the canonical subject references, creates the job, and returns a
    /// receipt without waiting for the matching result.
    pub async fn create(&self, request: MatchingCreate) -> Result<MatchingReceipt> {
        if request.group.is_empty() {
            return Err(ConduitError::invalid_request(
                "group must contain at least one subject",
            ));
        }

        let target = normalize_subject(&request.target)?;
        let mut group = Vec::with_capacity(request.group.len());
        for subject in &request.group {
            group.push(normalize_subject(subject)?);
        }
        ensure_unique_direct_entity_ids(&target, &group)?;

        let mut body = Map::new();
        body.insert(
            "context".to_string(),
            Value::String(request.context.as_str().to_string()),
        );
        body.insert("target".to_string(), target);
        body.insert("group".to_string(), Value::Array(group));
        if let Some(webhook) = request.webhook.as_ref() {
            body.insert("webhook".to_string(), normalize_webhook(webhook)?);
        }

        let response = self
            .transport
            .request(
                Method::POST,
                "/v1/matching/jobs",
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
        Ok(MatchingReceipt {
            job_id: receipt.job_id.clone(),
            status: receipt.status,
            stage: receipt.stage,
            estimated_wait_sec: receipt.estimated_wait_sec,
            handle: MatchingHandle {
                job_id: receipt.job_id,
                jobs: self.jobs.clone(),
                matching: self.clone(),
            },
        })
    }

    /// Fetches a completed matching result by identifier.
    pub async fn get(&self, matching_id: &str) -> Result<Matching> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/matching/{}", path_segment(matching_id, "matching_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_matching(&response.body)
    }

    /// Fetches the completed matching result associated with a job, if one exists yet.
    pub async fn get_by_job(&self, job_id: &str) -> Result<Option<Matching>> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/matching/by-job/{}", path_segment(job_id, "job_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        if response.body.is_empty() || String::from_utf8_lossy(&response.body).trim() == "null" {
            return Ok(None);
        }
        Ok(Some(parse_matching(&response.body)?))
    }
}

impl MatchingHandle {
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
    pub async fn wait(&self) -> Result<Matching> {
        self.wait_with(WaitOptions::default()).await
    }

    /// Waits for matching completion with a custom timeout.
    pub async fn wait_for(&self, timeout: Duration) -> Result<Matching> {
        self.wait_with(WaitOptions::default().timeout(timeout))
            .await
    }

    /// Waits for matching completion using custom polling options.
    pub async fn wait_with(&self, options: WaitOptions) -> Result<Matching> {
        let job = self.jobs.wait(&self.job_id, options).await?;
        match job.matching_id {
            Some(matching_id) => self.matching.get(&matching_id).await,
            None => Err(ConduitError::invalid_response(format!(
                "job {} succeeded but no matchingId was returned",
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

    /// Fetches the matching result if the job has already completed successfully.
    pub async fn matching(&self) -> Result<Option<Matching>> {
        self.matching.get_by_job(&self.job_id).await
    }
}

fn normalize_subject(subject: &SubjectRef) -> Result<Value> {
    let mut payload = Map::new();
    match subject {
        SubjectRef::Entity { entity_id } => {
            payload.insert("type".to_string(), Value::String("entity_id".to_string()));
            payload.insert(
                "entityId".to_string(),
                Value::String(require_non_empty(entity_id, "subject.entityId")?),
            );
        }
        SubjectRef::Media { media_id, selector } => {
            payload.insert(
                "type".to_string(),
                Value::String("media_target".to_string()),
            );
            payload.insert(
                "mediaId".to_string(),
                Value::String(require_non_empty(media_id, "subject.mediaId")?),
            );
            payload.insert("selector".to_string(), normalize_target(selector)?);
        }
    }
    Ok(Value::Object(payload))
}

fn ensure_unique_direct_entity_ids(target: &Value, group: &[Value]) -> Result<()> {
    let mut seen = HashSet::new();
    for subject in std::iter::once(target).chain(group.iter()) {
        let Some(object) = subject.as_object() else {
            continue;
        };
        let Some(entity_id) = object.get("entityId").and_then(Value::as_str) else {
            continue;
        };
        if object.get("mediaId").is_some() {
            continue;
        }
        if !seen.insert(entity_id.to_string()) {
            return Err(ConduitError::invalid_request(
                "target and group must reference different direct entity IDs",
            ));
        }
    }
    Ok(())
}
