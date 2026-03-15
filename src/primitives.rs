//! Advanced primitives for media, jobs, and entities.

use crate::common::{
    content_type_from_name, file_name_from_url, parse_http_url, path_segment, require_non_empty,
    resolve_label, strip_content_type,
};
use crate::error::{ConduitError, Result};
use crate::model::{
    Entity, FileDeleteReceipt, Job, JobEvent, JobEventKind, JobStatus, ListEntitiesResponse,
    ListFilesResponse, MediaFile, MediaObject, RetentionLockResult, parse_delete_receipt,
    parse_entity, parse_job, parse_list_entities, parse_list_files, parse_media_file,
    parse_media_object, parse_retention_lock,
};
use crate::transport::{MultipartFile, RequestBody, RequestOptions, Transport};
use async_stream::try_stream;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use reqwest::Method;
use reqwest::redirect::Policy;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Callback invoked for each polling event emitted by `wait()` and `stream()` helpers.
pub type JobEventCallback = Arc<dyn Fn(JobEvent) + Send + Sync>;

#[derive(Debug, Clone, Default)]
/// Additional options shared by cancel and other action-style APIs.
pub struct ActionOptions {
    /// Optional idempotency key applied to the request.
    pub idempotency_key: Option<String>,
    /// Optional request identifier echoed by the API.
    pub request_id: Option<String>,
}

impl ActionOptions {
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

#[derive(Clone, Default)]
/// Polling configuration used by `wait()` helpers.
pub struct WaitOptions {
    /// Maximum time spent polling before returning a timeout error.
    pub timeout: Option<Duration>,
    /// Delay between status polls.
    pub poll_interval: Option<Duration>,
    /// Optional callback invoked for every emitted [`JobEvent`].
    pub on_event: Option<JobEventCallback>,
}

impl WaitOptions {
    /// Sets the maximum wait duration.
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    /// Sets the interval between polls.
    pub fn poll_interval(mut self, value: Duration) -> Self {
        self.poll_interval = Some(value);
        self
    }

    /// Registers a callback for each emitted event.
    pub fn on_event(mut self, value: JobEventCallback) -> Self {
        self.on_event = Some(value);
        self
    }
}

/// Polling configuration used by streaming helpers.
pub type StreamOptions = WaitOptions;

#[derive(Debug, Clone)]
/// Upload or reference source used by reports and media primitives.
pub enum Source {
    /// Reference media that already exists in Conduit.
    MediaId {
        /// Existing media identifier.
        media_id: String,
    },
    /// Upload bytes already available in memory.
    File {
        /// File name reported to the API for content type inference.
        file_name: String,
        /// Full file contents.
        data: Vec<u8>,
        /// Optional caller-supplied label.
        label: Option<String>,
    },
    /// Fetch media from a remote HTTP(S) URL, then upload it to Conduit.
    Url {
        /// Remote HTTP(S) URL.
        url: String,
        /// Optional caller-supplied label.
        label: Option<String>,
    },
    /// Read media from the local filesystem, then upload it to Conduit.
    Path {
        /// Path to the local file.
        path: PathBuf,
        /// Optional caller-supplied label.
        label: Option<String>,
    },
}

impl Source {
    /// Creates a source that references existing uploaded media.
    pub fn media_id(value: impl Into<String>) -> Self {
        Self::MediaId {
            media_id: value.into(),
        }
    }

    /// Creates a source from in-memory bytes.
    pub fn file(file_name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self::File {
            file_name: file_name.into(),
            data: data.into(),
            label: None,
        }
    }

    /// Creates a source from a remote HTTP(S) URL.
    pub fn url(value: impl Into<String>) -> Self {
        Self::Url {
            url: value.into(),
            label: None,
        }
    }

    /// Creates a source from a local filesystem path.
    pub fn path(value: impl Into<PathBuf>) -> Self {
        Self::Path {
            path: value.into(),
            label: None,
        }
    }

    /// Applies a user-facing label to the source when supported.
    pub fn with_label(self, label: impl Into<String>) -> Self {
        let label = Some(label.into());
        match self {
            Self::MediaId { media_id } => Self::MediaId { media_id },
            Self::File {
                file_name, data, ..
            } => Self::File {
                file_name,
                data,
                label,
            },
            Self::Url { url, .. } => Self::Url { url, label },
            Self::Path { path, .. } => Self::Path { path, label },
        }
    }
}

#[derive(Debug, Clone)]
/// Advanced stable surface for low-level entities, media, and jobs APIs.
pub struct PrimitivesResource {
    /// Stable entity operations.
    pub entities: EntitiesResource,
    /// Media upload and management operations.
    pub media: MediaResource,
    /// Job inspection and cancellation operations.
    pub jobs: JobsResource,
}

#[derive(Debug, Clone)]
/// Resource group for job inspection, cancellation, and polling.
pub struct JobsResource {
    transport: Arc<Transport>,
    poll_interval: Duration,
}

#[derive(Debug, Clone)]
/// Resource group for stable entity reads and updates.
pub struct EntitiesResource {
    transport: Arc<Transport>,
}

#[derive(Debug, Clone)]
/// Resource group for low-level media upload and management.
pub struct MediaResource {
    transport: Arc<Transport>,
    timeout: Duration,
    max_source_bytes: u64,
}

#[derive(Debug, Clone)]
struct UploadMaterialization {
    payload: Vec<u8>,
    file_name: String,
    label: String,
    content_type: Option<String>,
}

impl JobsResource {
    pub(crate) fn new(transport: Arc<Transport>, poll_interval: Duration) -> Self {
        Self {
            transport,
            poll_interval,
        }
    }

    /// Fetches the latest state of a job.
    pub async fn get(&self, job_id: &str) -> Result<Job> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/jobs/{}", path_segment(job_id, "job_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_job(&response.body)
    }

    /// Requests cancellation for the given job.
    pub async fn cancel(&self, job_id: &str) -> Result<Job> {
        self.cancel_with(job_id, ActionOptions::default()).await
    }

    /// Requests cancellation using custom action options.
    pub async fn cancel_with(&self, job_id: &str, options: ActionOptions) -> Result<Job> {
        let response = self
            .transport
            .request(
                Method::POST,
                &format!("/v1/jobs/{}/cancel", path_segment(job_id, "job_id")?),
                RequestOptions {
                    idempotency_key: options.idempotency_key,
                    request_id: options.request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_job(&response.body)
    }

    /// Polls a job and yields status, stage, and terminal events until completion.
    pub fn stream(
        &self,
        job_id: impl Into<String>,
        options: StreamOptions,
    ) -> BoxStream<'static, Result<JobEvent>> {
        let job_id = job_id.into();
        let jobs = self.clone();
        let poll_interval = options.poll_interval.unwrap_or(self.poll_interval);
        let timeout = options.timeout.unwrap_or(Duration::from_secs(300));
        let on_event = options.on_event.clone();

        Box::pin(try_stream! {
            let validated_job_id = require_non_empty(&job_id, "job_id")?;
            let deadline = tokio::time::Instant::now() + timeout;
            let mut last_status = None;
            let mut last_stage = None;

            loop {
                if tokio::time::Instant::now() > deadline {
                    Err(ConduitError::timeout(
                        format!("timed out waiting for job {validated_job_id} after {}ms", timeout.as_millis()),
                        None,
                    ))?;
                }

                let job = jobs.get(&validated_job_id).await.map_err(|error| {
                    ConduitError::stream(
                        format!("failed to fetch status for job {validated_job_id}"),
                        Some(validated_job_id.clone()),
                    )
                    .with_source(error)
                })?;

                if last_status != Some(job.status) {
                    let event = JobEvent {
                        kind: JobEventKind::Status,
                        job: job.clone(),
                        stage: None,
                        progress: None,
                    };
                    if let Some(callback) = &on_event {
                        callback(event.clone());
                    }
                    yield event;
                    last_status = Some(job.status);
                }

                if let Some(stage) = job.stage
                    && last_stage != Some(stage)
                {
                    let event = JobEvent {
                        kind: JobEventKind::Stage,
                        job: job.clone(),
                        stage: Some(stage),
                        progress: job.progress,
                    };
                    if let Some(callback) = &on_event {
                        callback(event.clone());
                    }
                    yield event;
                    last_stage = Some(stage);
                }

                if job.status.is_terminal() {
                    let event = JobEvent {
                        kind: JobEventKind::Terminal,
                        job,
                        stage: None,
                        progress: None,
                    };
                    if let Some(callback) = &on_event {
                        callback(event.clone());
                    }
                    yield event;
                    break;
                }

                tokio::time::sleep(poll_interval).await;
            }
        })
    }

    /// Waits until a job reaches a terminal state.
    ///
    /// On success, returns the final [`Job`]. On failure or cancellation, returns a typed SDK
    /// error that preserves the job identifier and any request identifier provided by the API.
    pub async fn wait(&self, job_id: &str, options: WaitOptions) -> Result<Job> {
        let mut stream = self.stream(job_id.to_string(), options);
        while let Some(event) = stream.next().await {
            let event = event?;
            if event.kind != JobEventKind::Terminal {
                continue;
            }
            return match event.job.status {
                JobStatus::Succeeded => Ok(event.job),
                JobStatus::Failed => {
                    let error = event.job.error.clone();
                    Err(ConduitError::job_failed(
                        job_id.to_string(),
                        event.job.request_id.clone(),
                        error
                            .as_ref()
                            .map(|value| value.code.clone())
                            .unwrap_or_else(|| "job_failed".to_string()),
                        error
                            .as_ref()
                            .map(|value| value.message.clone())
                            .unwrap_or_else(|| format!("job {job_id} failed")),
                    ))
                }
                JobStatus::Canceled => Err(ConduitError::job_canceled(
                    job_id.to_string(),
                    event.job.request_id.clone(),
                )),
                JobStatus::Queued | JobStatus::Running => Err(ConduitError::timeout(
                    format!("job {job_id} did not reach a terminal state"),
                    None,
                )),
            };
        }
        Err(ConduitError::timeout(
            format!("timed out waiting for job {job_id}"),
            None,
        ))
    }
}

impl EntitiesResource {
    pub(crate) fn new(transport: Arc<Transport>) -> Self {
        Self { transport }
    }

    /// Fetches a single entity by identifier.
    pub async fn get(&self, entity_id: &str) -> Result<Entity> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/entities/{}", path_segment(entity_id, "entity_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_entity(&response.body)
    }

    /// Lists entities using cursor pagination.
    ///
    /// `limit` defaults to `20` when omitted.
    pub async fn list(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<ListEntitiesResponse> {
        let mut query = vec![("limit".to_string(), limit.unwrap_or(20).to_string())];
        if let Some(cursor) = cursor.filter(|value| !value.trim().is_empty()) {
            query.push(("cursor".to_string(), cursor));
        }
        let response = self
            .transport
            .request(
                Method::GET,
                "/v1/entities",
                RequestOptions {
                    query,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_list_entities(&response.body)
    }

    /// Updates the label associated with an entity.
    pub async fn update(
        &self,
        entity_id: &str,
        label: Option<String>,
        request_id: Option<String>,
    ) -> Result<Entity> {
        let response = self
            .transport
            .request(
                Method::PATCH,
                &format!("/v1/entities/{}", path_segment(entity_id, "entity_id")?),
                RequestOptions {
                    body: RequestBody::Json(json!({ "label": label })),
                    request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_entity(&response.body)
    }
}

impl MediaResource {
    pub(crate) fn new(transport: Arc<Transport>, timeout: Duration, max_source_bytes: u64) -> Self {
        Self {
            transport,
            timeout,
            max_source_bytes,
        }
    }

    /// Uploads media using any upload-capable [`Source`] variant.
    ///
    /// `Source::MediaId` is rejected because upload operations require new media bytes.
    pub async fn upload(
        &self,
        source: Source,
        idempotency_key: Option<String>,
        request_id: Option<String>,
    ) -> Result<MediaObject> {
        let materialized = self.materialize_source(source).await?;
        let response = self
            .transport
            .request(
                Method::POST,
                "/v1/files",
                RequestOptions {
                    body: RequestBody::Multipart {
                        fields: vec![("label".to_string(), materialized.label)],
                        file: MultipartFile {
                            file_name: materialized.file_name,
                            content_type: materialized.content_type,
                            payload: materialized.payload,
                        },
                    },
                    idempotency_key,
                    request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_media_object(&response.body)
    }

    /// Fetches a single media record by identifier.
    pub async fn get(&self, media_id: &str) -> Result<MediaFile> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!("/v1/files/{}", path_segment(media_id, "media_id")?),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_media_file(&response.body)
    }

    /// Lists media using cursor pagination.
    ///
    /// `limit` defaults to `20` when omitted.
    pub async fn list(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
        include_deleted: bool,
    ) -> Result<ListFilesResponse> {
        let mut query = vec![
            ("includeDeleted".to_string(), include_deleted.to_string()),
            ("limit".to_string(), limit.unwrap_or(20).to_string()),
        ];
        if let Some(cursor) = cursor.filter(|value| !value.trim().is_empty()) {
            query.push(("cursor".to_string(), cursor));
        }
        let response = self
            .transport
            .request(
                Method::GET,
                "/v1/files",
                RequestOptions {
                    query,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_list_files(&response.body)
    }

    /// Deletes a media object.
    pub async fn delete(
        &self,
        media_id: &str,
        idempotency_key: Option<String>,
        request_id: Option<String>,
    ) -> Result<FileDeleteReceipt> {
        let response = self
            .transport
            .request(
                Method::DELETE,
                &format!("/v1/files/{}", path_segment(media_id, "media_id")?),
                RequestOptions {
                    idempotency_key,
                    request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_delete_receipt(&response.body)
    }

    /// Enables or disables the retention lock for a media object.
    pub async fn set_retention_lock(
        &self,
        media_id: &str,
        locked: bool,
        request_id: Option<String>,
    ) -> Result<RetentionLockResult> {
        let response = self
            .transport
            .request(
                Method::PATCH,
                &format!(
                    "/v1/files/{}/retention",
                    path_segment(media_id, "media_id")?
                ),
                RequestOptions {
                    body: RequestBody::Json(json!({ "lock": locked })),
                    request_id,
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_retention_lock(&response.body)
    }

    pub(crate) async fn resolve_report_source(
        &self,
        source: Source,
        idempotency_key: Option<String>,
        request_id: Option<String>,
    ) -> Result<String> {
        match source {
            Source::MediaId { media_id } => require_non_empty(&media_id, "source.mediaId"),
            other => {
                let media = self.upload(other, idempotency_key, request_id).await?;
                Ok(media.media_id)
            }
        }
    }

    async fn materialize_source(&self, source: Source) -> Result<UploadMaterialization> {
        match source {
            Source::MediaId { .. } => Err(ConduitError::invalid_source(
                "upload source cannot use mediaId",
            )),
            Source::File {
                file_name,
                data,
                label,
            } => self.materialize_bytes(&file_name, data, label.as_deref()),
            Source::Url { url, label } => self.materialize_url(&url, label.as_deref()).await,
            Source::Path { path, label } => self.materialize_path(&path, label.as_deref()).await,
        }
    }

    async fn materialize_path(
        &self,
        path: &Path,
        label: Option<&str>,
    ) -> Result<UploadMaterialization> {
        let raw_path = path
            .to_str()
            .ok_or_else(|| ConduitError::invalid_source("source.path must be valid UTF-8"))?;
        require_non_empty(raw_path, "source.path")?;
        let metadata = tokio::fs::metadata(path).await.map_err(|error| {
            ConduitError::invalid_source(format!("file not found: {}", path.display()))
                .with_source(error)
        })?;
        if metadata.is_dir() {
            return Err(ConduitError::invalid_source(format!(
                "path is a directory: {}",
                path.display()
            )));
        }
        if metadata.len() > self.max_source_bytes {
            return Err(ConduitError::source_too_large(
                "source.path exceeds upload size limit",
            ));
        }
        let payload = tokio::fs::read(path).await.map_err(|error| {
            ConduitError::base("failed to read source.path", "source_error").with_source(error)
        })?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("upload.bin")
            .to_string();
        Ok(UploadMaterialization {
            content_type: content_type_from_name(&file_name),
            file_name: file_name.clone(),
            label: resolve_label(label, &file_name, "upload"),
            payload,
        })
    }

    fn materialize_bytes(
        &self,
        file_name: &str,
        data: Vec<u8>,
        label: Option<&str>,
    ) -> Result<UploadMaterialization> {
        if data.is_empty() {
            return Err(ConduitError::invalid_source("source.file is required"));
        }
        if data.len() as u64 > self.max_source_bytes {
            return Err(ConduitError::source_too_large(
                "source.file exceeds upload size limit",
            ));
        }
        let file_name = if file_name.trim().is_empty() {
            "upload.bin".to_string()
        } else {
            file_name.trim().to_string()
        };
        Ok(UploadMaterialization {
            content_type: content_type_from_name(&file_name),
            label: resolve_label(label, &file_name, "upload"),
            file_name,
            payload: data,
        })
    }

    async fn materialize_url(
        &self,
        raw_url: &str,
        label: Option<&str>,
    ) -> Result<UploadMaterialization> {
        let url = parse_http_url(raw_url, "source.url")?;
        let client = reqwest::Client::builder()
            .redirect(Policy::limited(5))
            .timeout(self.timeout)
            .build()
            .map_err(|error| {
                ConduitError::remote_fetch(
                    "remote fetch failed",
                    "remote_fetch_failed",
                    Some(url.to_string()),
                    None,
                )
                .with_source(error)
            })?;
        let mut response = client.get(url.clone()).send().await.map_err(|error| {
            if error.is_timeout() {
                return ConduitError::remote_fetch_timeout(Some(url.to_string()), None)
                    .with_source(error);
            }
            if error.is_redirect() {
                return ConduitError::remote_fetch(
                    "remote fetch exceeded redirect limit",
                    "remote_fetch_redirects_exhausted",
                    Some(url.to_string()),
                    None,
                )
                .with_source(error);
            }
            ConduitError::remote_fetch(
                "remote fetch failed",
                "remote_fetch_failed",
                Some(url.to_string()),
                None,
            )
            .with_source(error)
        })?;
        let status = response.status().as_u16();
        if !response.status().is_success() {
            return Err(ConduitError::remote_fetch(
                format!("remote fetch failed with status {status}"),
                "remote_fetch_failed",
                Some(url.to_string()),
                Some(status),
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > self.max_source_bytes)
        {
            return Err(ConduitError::remote_fetch_too_large(
                Some(url.to_string()),
                Some(status),
            ));
        }
        let mut payload = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            ConduitError::remote_fetch(
                "remote fetch failed",
                "remote_fetch_failed",
                Some(url.to_string()),
                Some(status),
            )
            .with_source(error)
        })? {
            payload.extend_from_slice(&chunk);
            if payload.len() as u64 > self.max_source_bytes {
                return Err(ConduitError::remote_fetch_too_large(
                    Some(url.to_string()),
                    Some(status),
                ));
            }
        }
        let final_url = response.url().clone();
        let file_name = file_name_from_url(&final_url);
        Ok(UploadMaterialization {
            content_type: strip_content_type(
                response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok()),
            ),
            file_name: file_name.clone(),
            label: resolve_label(label, &file_name, "remote"),
            payload,
        })
    }
}
