use crate::common::{path_segment, require_non_empty};
use crate::error::{ConduitError, Result};
use crate::model::{Job, JobEvent, MatchingAnalysisResponse, parse_job_receipt, parse_matching};
use crate::primitives::{JobsResource, StreamOptions, WaitOptions};
use crate::reports::{TargetSelector, Webhook, normalize_target_selector, normalize_webhook};
use crate::transport::{RequestBody, RequestOptions, Transport};
use futures_util::stream::BoxStream;
use reqwest::Method;
use serde_json::{Map, Value};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingContext {
    HiringTeamFit,
}

impl MatchingContext {
    fn as_str(self) -> &'static str {
        match self {
            Self::HiringTeamFit => "hiring_team_fit",
        }
    }
}

#[derive(Debug, Clone)]
pub enum MatchingSubject {
    EntityId(String),
    Media {
        media_id: String,
        selector: TargetSelector,
    },
}

impl MatchingSubject {
    pub fn entity_id(value: impl Into<String>) -> Self {
        Self::EntityId(value.into())
    }

    pub fn media(media_id: impl Into<String>, selector: TargetSelector) -> Self {
        Self::Media {
            media_id: media_id.into(),
            selector,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateMatchingRequest {
    pub context: MatchingContext,
    pub target: MatchingSubject,
    pub group: Vec<MatchingSubject>,
    pub webhook: Option<Webhook>,
    pub idempotency_key: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchingJobReceipt {
    pub job_id: String,
    pub status: String,
    pub handle: MatchingRunHandle,
    pub stage: Option<String>,
    pub estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MatchingRunHandle {
    job_id: String,
    jobs: JobsResource,
    matching: MatchingResource,
}

#[derive(Debug, Clone)]
pub struct MatchingResource {
    transport: Arc<Transport>,
    jobs: JobsResource,
}

impl MatchingResource {
    pub(crate) fn new(transport: Arc<Transport>, jobs: JobsResource) -> Self {
        Self { transport, jobs }
    }

    pub async fn create(&self, request: CreateMatchingRequest) -> Result<MatchingJobReceipt> {
        if request.group.is_empty() {
            return Err(ConduitError::invalid_request(
                "group must contain at least one subject",
            ));
        }
        let target = normalize_matching_subject(&request.target)?;
        let mut group = Vec::with_capacity(request.group.len());
        for subject in &request.group {
            group.push(normalize_matching_subject(subject)?);
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
        Ok(MatchingJobReceipt {
            job_id: receipt.job_id.clone(),
            status: receipt.status,
            stage: receipt.stage,
            estimated_wait_sec: receipt.estimated_wait_sec,
            handle: MatchingRunHandle {
                job_id: receipt.job_id,
                jobs: self.jobs.clone(),
                matching: self.clone(),
            },
        })
    }

    pub async fn get(&self, matching_id: &str) -> Result<MatchingAnalysisResponse> {
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

    pub async fn get_by_job(&self, job_id: &str) -> Result<Option<MatchingAnalysisResponse>> {
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

impl MatchingRunHandle {
    pub fn stream(&self, options: StreamOptions) -> BoxStream<'static, Result<JobEvent>> {
        self.jobs.stream(self.job_id.clone(), options)
    }

    pub async fn wait(&self, options: WaitOptions) -> Result<MatchingAnalysisResponse> {
        let job = self.jobs.wait(&self.job_id, options).await?;
        match job.matching_id {
            Some(matching_id) => self.matching.get(&matching_id).await,
            None => Err(ConduitError::invalid_response(format!(
                "job {} succeeded but no matchingId was returned",
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

    pub async fn matching(&self) -> Result<Option<MatchingAnalysisResponse>> {
        self.matching.get_by_job(&self.job_id).await
    }
}

fn normalize_matching_subject(subject: &MatchingSubject) -> Result<Value> {
    let mut payload = Map::new();
    match subject {
        MatchingSubject::EntityId(entity_id) => {
            payload.insert("type".to_string(), Value::String("entity_id".to_string()));
            payload.insert(
                "entityId".to_string(),
                Value::String(require_non_empty(entity_id, "subject.entityId")?),
            );
        }
        MatchingSubject::Media { media_id, selector } => {
            payload.insert(
                "type".to_string(),
                Value::String("media_target".to_string()),
            );
            payload.insert(
                "mediaId".to_string(),
                Value::String(require_non_empty(media_id, "subject.mediaId")?),
            );
            payload.insert("selector".to_string(), normalize_target_selector(selector)?);
        }
    }
    Ok(Value::Object(payload))
}

fn ensure_unique_direct_entity_ids(target: &Value, group: &[Value]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for subject in std::iter::once(target).chain(group.iter()) {
        let Some(object) = subject.as_object() else {
            continue;
        };
        if object.get("type").and_then(Value::as_str) != Some("entity_id") {
            continue;
        }
        let entity_id = object
            .get("entityId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !seen.insert(entity_id.to_string()) {
            return Err(ConduitError::invalid_request(
                "target and group must reference different direct entity IDs",
            ));
        }
    }
    Ok(())
}
