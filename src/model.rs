//! Typed response models returned by the Conduit API.

use crate::error::{ConduitError, Result};
use crate::matching::MatchingContext;
use crate::reports::ReportTemplate;
use serde::Deserialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone)]
/// Structured job error payload returned by the API.
pub struct JobErrorData {
    /// Stable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details returned by the API.
    pub details: Option<Box<Value>>,
    /// Whether the failure is marked retryable by the API.
    pub retryable: Option<bool>,
}

#[derive(Debug, Clone)]
/// Usage data associated with a completed job.
pub struct Usage {
    /// Gross credits used.
    pub credits_used: f64,
    /// Net credits used after discounts.
    pub credits_net_used: f64,
    /// Credits discounted, when reported.
    pub credits_discounted: Option<f64>,
    /// Processed media duration in milliseconds, when reported.
    pub duration_ms: Option<f64>,
    /// Model version identifier, when reported.
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reservation lifecycle for job credit holds.
pub enum CreditReservationStatus {
    /// Credits are currently reserved.
    Active,
    /// Credits were released without being applied.
    Released,
    /// Credits were applied to the final usage record.
    Applied,
}

impl CreditReservationStatus {
    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "released" => Ok(Self::Released),
            "applied" => Ok(Self::Applied),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported reservation status"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
/// Credit reservation details attached to a job.
pub struct JobCreditReservation {
    /// Number of credits reserved for the job, when reported.
    pub reserved_credits: Option<f64>,
    /// Reservation lifecycle status, when reported.
    pub reservation_status: Option<CreditReservationStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Initial status returned in a create receipt.
pub enum ReceiptStatus {
    /// The job has been accepted and queued.
    Queued,
    /// The job has already started running.
    Running,
}

impl ReceiptStatus {
    /// Returns the canonical API identifier for the receipt status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
        }
    }

    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported receipt status"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Kind of long-running job tracked by Conduit.
pub enum JobKind {
    /// Report generation job.
    ReportGenerate,
    /// Matching generation job.
    MatchingGenerate,
}

impl JobKind {
    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "report.generate" => Ok(Self::ReportGenerate),
            "matching.generate" => Ok(Self::MatchingGenerate),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported job type"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Current lifecycle status of a job.
pub enum JobStatus {
    /// The job has been accepted and queued.
    Queued,
    /// The job is actively running.
    Running,
    /// The job completed successfully.
    Succeeded,
    /// The job completed with a failure.
    Failed,
    /// The job was canceled.
    Canceled,
}

impl JobStatus {
    /// Returns the canonical API identifier for the job status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    /// Returns `true` when the status is terminal.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }

    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "canceled" => Ok(Self::Canceled),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported job status"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Best-effort stage within a long-running job.
pub enum JobStage {
    /// Media has been uploaded.
    Uploaded,
    /// The job is queued.
    Queued,
    /// Media is being transcoded.
    Transcoding,
    /// Target extraction is in progress.
    Extracting,
    /// Behavioral scoring is in progress.
    Scoring,
    /// Output rendering is in progress.
    Rendering,
    /// Final job finalization is in progress.
    Finalizing,
}

impl JobStage {
    /// Returns the canonical API identifier for the job stage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uploaded => "uploaded",
            Self::Queued => "queued",
            Self::Transcoding => "transcoding",
            Self::Extracting => "extracting",
            Self::Scoring => "scoring",
            Self::Rendering => "rendering",
            Self::Finalizing => "finalizing",
        }
    }

    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "uploaded" => Ok(Self::Uploaded),
            "queued" => Ok(Self::Queued),
            "transcoding" => Ok(Self::Transcoding),
            "extracting" => Ok(Self::Extracting),
            "scoring" => Ok(Self::Scoring),
            "rendering" => Ok(Self::Rendering),
            "finalizing" => Ok(Self::Finalizing),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: unsupported job stage"
            ))),
        }
    }
}

fn parse_job_stage(value: &str) -> Option<JobStage> {
    JobStage::parse(value, "job.stage").ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Event kind emitted by polling helpers.
pub enum JobEventKind {
    /// A status transition was observed.
    Status,
    /// A stage transition was observed.
    Stage,
    /// A terminal state was observed.
    Terminal,
}

impl JobEventKind {
    /// Returns the canonical event kind identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Stage => "stage",
            Self::Terminal => "terminal",
        }
    }
}

#[derive(Debug, Clone)]
/// Long-running job record returned by jobs and handle helpers.
pub struct Job {
    /// Job identifier.
    pub id: String,
    /// Job kind.
    pub kind: JobKind,
    /// Current lifecycle status.
    pub status: JobStatus,
    /// Job creation timestamp.
    pub created_at: OffsetDateTime,
    /// Last update timestamp.
    pub updated_at: OffsetDateTime,
    /// Current stage, when reported.
    pub stage: Option<JobStage>,
    /// Advisory progress value, when reported.
    pub progress: Option<f64>,
    /// Completed report identifier for successful report jobs.
    pub report_id: Option<String>,
    /// Completed matching identifier for successful matching jobs.
    pub matching_id: Option<String>,
    /// Usage details, when reported.
    pub usage: Option<Usage>,
    /// Credit reservation details, when reported.
    pub credits: Option<JobCreditReservation>,
    /// Released credits, when reported.
    pub released_credits: Option<f64>,
    /// Structured failure payload for failed jobs.
    pub error: Option<JobErrorData>,
    /// Request identifier echoed by the API, when available.
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
/// Event emitted by polling helpers such as `stream()` and `wait()` callbacks.
pub struct JobEvent {
    /// Event kind.
    pub kind: JobEventKind,
    /// Latest job snapshot.
    pub job: Job,
    /// Stage carried by stage events.
    pub stage: Option<JobStage>,
    /// Progress value carried by stage events, when available.
    pub progress: Option<f64>,
}

#[derive(Debug, Clone)]
/// Rendered report output payload.
pub struct ReportOutputData {
    /// Generated template identifier.
    pub template: ReportTemplate,
    /// Markdown rendering, when available.
    pub markdown: Option<String>,
    /// Structured JSON output, when available.
    pub json: Option<Value>,
    /// Hosted report URL, when available.
    pub report_url: Option<String>,
}

#[derive(Debug, Clone)]
/// Completed report record.
pub struct Report {
    /// Report identifier.
    pub id: String,
    /// Report creation timestamp.
    pub created_at: OffsetDateTime,
    /// Rendered output representations.
    pub output: ReportOutputData,
    /// Originating job identifier, when reported.
    pub job_id: Option<String>,
    /// Report label, when reported.
    pub label: Option<String>,
    /// Resolved entity identifier, when reported.
    pub entity_id: Option<String>,
    /// Resolved entity label, when reported.
    pub entity_label: Option<String>,
    /// Source media identifier, when reported.
    pub media_id: Option<String>,
}

#[derive(Debug, Clone)]
/// Resolved subject returned inside a matching response.
pub struct MatchingResolvedSubject {
    /// Original source reference preserved as JSON.
    pub source: Value,
    /// Resolved stable entity identifier, when available.
    pub entity_id: Option<String>,
    /// Resolved display label, when available.
    pub resolved_label: Option<String>,
}

#[derive(Debug, Clone)]
/// Rendered matching output payload.
pub struct MatchingOutputData {
    /// Markdown rendering, when available.
    pub markdown: Option<String>,
    /// Structured JSON output, when available.
    pub json: Option<Value>,
}

#[derive(Debug, Clone)]
/// Completed matching result record.
pub struct Matching {
    /// Matching identifier.
    pub id: String,
    /// Matching creation timestamp.
    pub created_at: OffsetDateTime,
    /// Matching context.
    pub context: MatchingContext,
    /// Rendered output representations.
    pub output: MatchingOutputData,
    /// Originating job identifier, when reported.
    pub job_id: Option<String>,
    /// Matching label, when reported.
    pub label: Option<String>,
    /// Resolved target subject, when reported.
    pub target: Option<MatchingResolvedSubject>,
    /// Resolved comparison group.
    pub group: Vec<MatchingResolvedSubject>,
}

#[derive(Debug, Clone)]
/// Minimal media record returned by upload operations.
pub struct MediaObject {
    /// Media identifier.
    pub media_id: String,
    /// Media creation timestamp.
    pub created_at: OffsetDateTime,
    /// Content type recorded for the media.
    pub content_type: String,
    /// User-facing label.
    pub label: String,
    /// Size in bytes, when reported.
    pub size_bytes: Option<u64>,
    /// Duration in seconds, when reported.
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
/// Retention metadata attached to a media record.
pub struct MediaRetention {
    /// Expiration timestamp, when reported.
    pub expires_at: Option<OffsetDateTime>,
    /// Remaining retention days, when reported.
    pub days_remaining: Option<u64>,
    /// Whether a retention lock is active.
    pub locked: bool,
}

#[derive(Debug, Clone)]
/// Detailed media record returned by `media.get()` and `media.list()`.
pub struct MediaFile {
    /// Media identifier.
    pub media_id: String,
    /// Media creation timestamp.
    pub created_at: OffsetDateTime,
    /// Content type recorded for the media.
    pub content_type: String,
    /// User-facing label.
    pub label: String,
    /// Processing status reported by the API.
    pub processing_status: String,
    /// Last usage timestamp, when reported.
    pub last_used_at: Option<OffsetDateTime>,
    /// Retention metadata.
    pub retention: MediaRetention,
    /// Size in bytes, when reported.
    pub size_bytes: Option<u64>,
    /// Duration in seconds, when reported.
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
/// Receipt returned after deleting a media record.
pub struct FileDeleteReceipt {
    #[serde(rename = "mediaId")]
    /// Deleted media identifier.
    pub media_id: String,
    /// Whether the delete operation succeeded.
    pub deleted: bool,
}

#[derive(Debug, Clone, Deserialize)]
/// Response returned after updating a media retention lock.
pub struct RetentionLockResult {
    #[serde(rename = "mediaId")]
    /// Media identifier.
    pub media_id: String,
    #[serde(rename = "retentionLock")]
    /// Current retention lock state.
    pub retention_lock: bool,
    /// Human-readable API message.
    pub message: String,
}

#[derive(Debug, Clone)]
/// Cursor-paginated media list response.
pub struct ListFilesResponse {
    /// Returned media items.
    pub files: Vec<MediaFile>,
    /// Whether another page is available.
    pub has_more: bool,
    /// Cursor for the next page, when available.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
/// Stable entity record.
pub struct Entity {
    /// Entity identifier.
    pub id: String,
    /// Entity creation timestamp.
    pub created_at: OffsetDateTime,
    /// Current entity label, when available.
    pub label: Option<String>,
    /// Number of media records linked to the entity.
    pub media_count: f64,
    /// Last time the entity was observed, when reported.
    pub last_seen_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
/// Cursor-paginated entity list response.
pub struct ListEntitiesResponse {
    /// Returned entity items.
    pub entities: Vec<Entity>,
    /// Whether another page is available.
    pub has_more: bool,
    /// Cursor for the next page, when available.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct JobReceipt {
    pub job_id: String,
    pub status: ReceiptStatus,
    pub stage: Option<JobStage>,
    pub estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct JobWire {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    stage: Option<String>,
    progress: Option<f64>,
    #[serde(rename = "reportId")]
    report_id: Option<String>,
    #[serde(rename = "matchingId")]
    matching_id: Option<String>,
    usage: Option<UsageWire>,
    credits: Option<JobCreditsWire>,
    #[serde(rename = "releasedCredits")]
    released_credits: Option<f64>,
    error: Option<JobErrorWire>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobErrorWire {
    code: String,
    message: String,
    details: Option<Value>,
    retryable: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UsageWire {
    #[serde(rename = "creditsUsed")]
    credits_used: f64,
    #[serde(rename = "creditsNetUsed")]
    credits_net_used: f64,
    #[serde(rename = "creditsDiscounted")]
    credits_discounted: Option<f64>,
    #[serde(rename = "durationMs")]
    duration_ms: Option<f64>,
    #[serde(rename = "modelVersion")]
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobCreditsWire {
    #[serde(rename = "reservedCredits")]
    reserved_credits: Option<f64>,
    #[serde(rename = "reservationStatus")]
    reservation_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JobReceiptWire {
    #[serde(rename = "jobId")]
    job_id: String,
    status: String,
    stage: Option<String>,
    #[serde(rename = "estimatedWaitSec")]
    estimated_wait_sec: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ReportWire {
    id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "jobId")]
    job_id: Option<String>,
    label: Option<String>,
    entity: Option<ReportEntityWire>,
    media: Option<ReportMediaWire>,
    output: ReportOutputWire,
    markdown: Option<String>,
    json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ReportEntityWire {
    id: String,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReportMediaWire {
    #[serde(rename = "mediaId")]
    media_id: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReportOutputWire {
    template: String,
}

#[derive(Debug, Deserialize)]
struct MatchingWire {
    id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    context: String,
    #[serde(rename = "jobId")]
    job_id: Option<String>,
    label: Option<String>,
    target: Option<MatchingSubjectWire>,
    group: Option<Vec<MatchingSubjectWire>>,
    markdown: Option<String>,
    json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct MatchingSubjectWire {
    source: Value,
    #[serde(rename = "entityId")]
    entity_id: Option<String>,
    #[serde(rename = "resolvedLabel")]
    resolved_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaObjectWire {
    #[serde(rename = "mediaId")]
    media_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "contentType")]
    content_type: String,
    label: String,
    #[serde(rename = "sizeBytes")]
    size_bytes: Option<u64>,
    #[serde(rename = "durationSeconds")]
    duration_seconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct MediaFileWire {
    #[serde(rename = "mediaId")]
    media_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "contentType")]
    content_type: String,
    label: String,
    #[serde(rename = "processingStatus")]
    processing_status: String,
    #[serde(rename = "lastUsedAt")]
    last_used_at: Option<String>,
    retention: MediaRetentionWire,
    #[serde(rename = "sizeBytes")]
    size_bytes: Option<u64>,
    #[serde(rename = "durationSeconds")]
    duration_seconds: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct MediaRetentionWire {
    #[serde(rename = "expiresAt")]
    expires_at: Option<String>,
    #[serde(rename = "daysRemaining")]
    days_remaining: Option<u64>,
    locked: bool,
}

#[derive(Debug, Deserialize)]
struct ListFilesWire {
    files: Vec<MediaFileWire>,
    #[serde(rename = "hasMore")]
    has_more: bool,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EntityWire {
    id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    label: Option<String>,
    #[serde(rename = "mediaCount")]
    media_count: f64,
    #[serde(rename = "lastSeenAt")]
    last_seen_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListEntitiesWire {
    entities: Vec<EntityWire>,
    #[serde(rename = "hasMore")]
    has_more: bool,
    cursor: Option<String>,
}

pub(crate) fn parse_job(payload: &[u8]) -> Result<Job> {
    let wire: JobWire = decode_json(payload)?;
    Ok(Job {
        id: response_string(&wire.id, "job.id")?,
        kind: JobKind::parse(&wire.kind, "job.type")?,
        status: JobStatus::parse(&wire.status, "job.status")?,
        created_at: response_datetime(&wire.created_at, "job.createdAt")?,
        updated_at: response_datetime(&wire.updated_at, "job.updatedAt")?,
        stage: wire
            .stage
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(parse_job_stage),
        progress: wire.progress,
        report_id: wire.report_id.filter(|value| !value.trim().is_empty()),
        matching_id: wire.matching_id.filter(|value| !value.trim().is_empty()),
        usage: wire.usage.map(|usage| Usage {
            credits_used: usage.credits_used,
            credits_net_used: usage.credits_net_used,
            credits_discounted: usage.credits_discounted,
            duration_ms: usage.duration_ms,
            model_version: usage.model_version.filter(|value| !value.trim().is_empty()),
        }),
        credits: wire
            .credits
            .map(|credits| -> Result<JobCreditReservation> {
                Ok(JobCreditReservation {
                    reserved_credits: credits.reserved_credits,
                    reservation_status: credits
                        .reservation_status
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(|value| {
                            CreditReservationStatus::parse(value, "job.credits.reservationStatus")
                        })
                        .transpose()?,
                })
            })
            .transpose()?,
        released_credits: wire.released_credits,
        error: wire.error.map(|error| JobErrorData {
            code: error.code,
            message: error.message,
            details: error.details.map(Box::new),
            retryable: error.retryable,
        }),
        request_id: wire.request_id.filter(|value| !value.trim().is_empty()),
    })
}

pub(crate) fn parse_job_receipt(payload: &[u8]) -> Result<JobReceipt> {
    let wire: JobReceiptWire = decode_json(payload)?;
    Ok(JobReceipt {
        job_id: response_string(&wire.job_id, "jobReceipt.jobId")?,
        status: ReceiptStatus::parse(&wire.status, "jobReceipt.status")?,
        stage: wire
            .stage
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(parse_job_stage),
        estimated_wait_sec: wire.estimated_wait_sec,
    })
}

pub(crate) fn parse_report(payload: &[u8]) -> Result<Report> {
    let wire: ReportWire = decode_json(payload)?;
    let media = wire.media.unwrap_or(ReportMediaWire {
        media_id: None,
        url: None,
    });
    Ok(Report {
        id: response_string(&wire.id, "report.id")?,
        created_at: response_datetime(&wire.created_at, "report.createdAt")?,
        output: ReportOutputData {
            template: ReportTemplate::parse(&wire.output.template, "report.output.template")?,
            markdown: wire.markdown.filter(|value| !value.trim().is_empty()),
            json: wire.json,
            report_url: media.url.filter(|value| !value.trim().is_empty()),
        },
        job_id: wire.job_id.filter(|value| !value.trim().is_empty()),
        label: wire.label.filter(|value| !value.trim().is_empty()),
        entity_id: wire
            .entity
            .as_ref()
            .and_then(|entity| (!entity.id.trim().is_empty()).then(|| entity.id.clone())),
        entity_label: wire
            .entity
            .and_then(|entity| entity.label.filter(|value| !value.trim().is_empty())),
        media_id: media.media_id.filter(|value| !value.trim().is_empty()),
    })
}

pub(crate) fn parse_matching(payload: &[u8]) -> Result<Matching> {
    let wire: MatchingWire = decode_json(payload)?;
    Ok(Matching {
        id: response_string(&wire.id, "matching.id")?,
        created_at: response_datetime(&wire.created_at, "matching.createdAt")?,
        context: MatchingContext::parse(&wire.context, "matching.context")?,
        output: MatchingOutputData {
            markdown: wire.markdown.filter(|value| !value.trim().is_empty()),
            json: wire.json,
        },
        job_id: wire.job_id.filter(|value| !value.trim().is_empty()),
        label: wire.label.filter(|value| !value.trim().is_empty()),
        target: wire.target.map(parse_matching_subject),
        group: wire
            .group
            .unwrap_or_default()
            .into_iter()
            .map(parse_matching_subject)
            .collect(),
    })
}

pub(crate) fn parse_media_object(payload: &[u8]) -> Result<MediaObject> {
    let wire: MediaObjectWire = decode_json(payload)?;
    Ok(MediaObject {
        media_id: response_string(&wire.media_id, "media.mediaId")?,
        created_at: response_datetime(&wire.created_at, "media.createdAt")?,
        content_type: response_string(&wire.content_type, "media.contentType")?,
        label: response_string(&wire.label, "media.label")?,
        size_bytes: wire.size_bytes,
        duration_seconds: wire.duration_seconds,
    })
}

pub(crate) fn parse_media_file(payload: &[u8]) -> Result<MediaFile> {
    let wire: MediaFileWire = decode_json(payload)?;
    parse_media_file_wire(wire)
}

fn parse_media_file_wire(wire: MediaFileWire) -> Result<MediaFile> {
    Ok(MediaFile {
        media_id: response_string(&wire.media_id, "media.mediaId")?,
        created_at: response_datetime(&wire.created_at, "media.createdAt")?,
        content_type: response_string(&wire.content_type, "media.contentType")?,
        label: response_string(&wire.label, "media.label")?,
        processing_status: response_string(&wire.processing_status, "media.processingStatus")?,
        last_used_at: optional_datetime(wire.last_used_at, "media.lastUsedAt")?,
        retention: MediaRetention {
            expires_at: optional_datetime(wire.retention.expires_at, "media.retention.expiresAt")?,
            days_remaining: wire.retention.days_remaining,
            locked: wire.retention.locked,
        },
        size_bytes: wire.size_bytes,
        duration_seconds: wire.duration_seconds,
    })
}

pub(crate) fn parse_list_files(payload: &[u8]) -> Result<ListFilesResponse> {
    let wire: ListFilesWire = decode_json(payload)?;
    let mut files = Vec::with_capacity(wire.files.len());
    for file in wire.files {
        files.push(parse_media_file_wire(file)?);
    }
    Ok(ListFilesResponse {
        files,
        has_more: wire.has_more,
        next_cursor: wire.next_cursor.filter(|value| !value.trim().is_empty()),
    })
}

pub(crate) fn parse_delete_receipt(payload: &[u8]) -> Result<FileDeleteReceipt> {
    let receipt: FileDeleteReceipt = decode_json(payload)?;
    response_string(&receipt.media_id, "delete.mediaId")?;
    Ok(receipt)
}

pub(crate) fn parse_retention_lock(payload: &[u8]) -> Result<RetentionLockResult> {
    let result: RetentionLockResult = decode_json(payload)?;
    response_string(&result.media_id, "retention.mediaId")?;
    response_string(&result.message, "retention.message")?;
    Ok(result)
}

pub(crate) fn parse_entity(payload: &[u8]) -> Result<Entity> {
    let wire: EntityWire = decode_json(payload)?;
    parse_entity_wire(wire)
}

fn parse_entity_wire(wire: EntityWire) -> Result<Entity> {
    Ok(Entity {
        id: response_string(&wire.id, "entity.id")?,
        created_at: response_datetime(&wire.created_at, "entity.createdAt")?,
        label: wire.label.filter(|value| !value.trim().is_empty()),
        media_count: wire.media_count,
        last_seen_at: optional_datetime(wire.last_seen_at, "entity.lastSeenAt")?,
    })
}

pub(crate) fn parse_list_entities(payload: &[u8]) -> Result<ListEntitiesResponse> {
    let wire: ListEntitiesWire = decode_json(payload)?;
    let mut entities = Vec::with_capacity(wire.entities.len());
    for entity in wire.entities {
        entities.push(parse_entity_wire(entity)?);
    }
    Ok(ListEntitiesResponse {
        entities,
        has_more: wire.has_more,
        next_cursor: wire.cursor.filter(|value| !value.trim().is_empty()),
    })
}

fn parse_matching_subject(wire: MatchingSubjectWire) -> MatchingResolvedSubject {
    MatchingResolvedSubject {
        source: wire.source,
        entity_id: wire.entity_id.filter(|value| !value.trim().is_empty()),
        resolved_label: wire.resolved_label.filter(|value| !value.trim().is_empty()),
    }
}

fn response_string(value: &str, name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConduitError::invalid_response(format!(
            "invalid {name}: expected string"
        )));
    }
    Ok(trimmed.to_string())
}

fn response_datetime(value: &str, name: &str) -> Result<OffsetDateTime> {
    let value = response_string(value, name)?;
    OffsetDateTime::parse(&value, &Rfc3339).map_err(|error| {
        ConduitError::invalid_response(format!("invalid {name}: expected ISO8601 string"))
            .with_source(error)
    })
}

fn optional_datetime(value: Option<String>, name: &str) -> Result<Option<OffsetDateTime>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| response_datetime(&value, name))
        .transpose()
}

fn decode_json<T>(payload: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(payload)
        .map_err(|error| ConduitError::invalid_response("invalid JSON response").with_source(error))
}
