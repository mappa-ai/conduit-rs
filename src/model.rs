use crate::common::{parse_iso8601, response_string};
use crate::error::{ConduitError, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct JobErrorData {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
    pub retryable: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub credits_used: f64,
    pub credits_net_used: f64,
    pub credits_discounted: Option<f64>,
    pub duration_ms: Option<f64>,
    pub model_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobCreditReservation {
    pub reserved_credits: Option<f64>,
    pub reservation_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub r#type: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub stage: Option<String>,
    pub progress: Option<f64>,
    pub report_id: Option<String>,
    pub matching_id: Option<String>,
    pub usage: Option<Usage>,
    pub credits: Option<JobCreditReservation>,
    pub released_credits: Option<f64>,
    pub error: Option<JobErrorData>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobEvent {
    pub r#type: String,
    pub job: Job,
    pub stage: Option<String>,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ReportOutputData {
    pub template: String,
    pub markdown: Option<String>,
    pub json: Option<Value>,
    pub report_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub id: String,
    pub created_at: String,
    pub output: ReportOutputData,
    pub job_id: Option<String>,
    pub label: Option<String>,
    pub entity_id: Option<String>,
    pub entity_label: Option<String>,
    pub media_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchingResolvedSubject {
    pub source: Value,
    pub entity_id: Option<String>,
    pub resolved_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MatchingOutputData {
    pub markdown: Option<String>,
    pub json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct MatchingAnalysisResponse {
    pub id: String,
    pub created_at: String,
    pub context: String,
    pub output: MatchingOutputData,
    pub job_id: Option<String>,
    pub label: Option<String>,
    pub target: Option<MatchingResolvedSubject>,
    pub group: Vec<MatchingResolvedSubject>,
}

#[derive(Debug, Clone)]
pub struct MediaObject {
    pub media_id: String,
    pub created_at: String,
    pub content_type: String,
    pub label: String,
    pub size_bytes: Option<u64>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MediaRetention {
    pub expires_at: Option<String>,
    pub days_remaining: Option<u64>,
    pub locked: bool,
}

#[derive(Debug, Clone)]
pub struct MediaFile {
    pub media_id: String,
    pub created_at: String,
    pub content_type: String,
    pub label: String,
    pub processing_status: String,
    pub last_used_at: Option<String>,
    pub retention: MediaRetention,
    pub size_bytes: Option<u64>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileDeleteReceipt {
    #[serde(rename = "mediaId")]
    pub media_id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetentionLockResult {
    #[serde(rename = "mediaId")]
    pub media_id: String,
    #[serde(rename = "retentionLock")]
    pub retention_lock: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ListFilesResponse {
    pub files: Vec<MediaFile>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub created_at: String,
    pub label: Option<String>,
    pub media_count: f64,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListEntitiesResponse {
    pub entities: Vec<Entity>,
    pub has_more: bool,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WebhookEvent {
    pub id: String,
    pub r#type: String,
    pub created_at: String,
    pub timestamp: String,
    pub data: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct JobReceipt {
    pub job_id: String,
    pub status: String,
    pub stage: Option<String>,
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

#[derive(Debug, Deserialize)]
struct WebhookEventWire {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    timestamp: String,
    data: Value,
}

pub(crate) fn parse_job(payload: &[u8]) -> Result<Job> {
    let wire: JobWire = decode_json(payload)?;
    Ok(Job {
        id: response_string(&wire.id, "job.id")?,
        r#type: response_string(&wire.kind, "job.type")?,
        status: response_string(&wire.status, "job.status")?,
        created_at: response_string(&wire.created_at, "job.createdAt")?,
        updated_at: response_string(&wire.updated_at, "job.updatedAt")?,
        stage: wire.stage.filter(|value| !value.trim().is_empty()),
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
        credits: wire.credits.map(|credits| JobCreditReservation {
            reserved_credits: credits.reserved_credits,
            reservation_status: credits
                .reservation_status
                .filter(|value| !value.trim().is_empty()),
        }),
        released_credits: wire.released_credits,
        error: wire.error.map(|error| JobErrorData {
            code: error.code,
            message: error.message,
            details: error.details,
            retryable: error.retryable,
        }),
        request_id: wire.request_id.filter(|value| !value.trim().is_empty()),
    })
}

pub(crate) fn parse_job_receipt(payload: &[u8]) -> Result<JobReceipt> {
    let wire: JobReceiptWire = decode_json(payload)?;
    Ok(JobReceipt {
        job_id: response_string(&wire.job_id, "jobReceipt.jobId")?,
        status: response_string(&wire.status, "jobReceipt.status")?,
        stage: wire.stage.filter(|value| !value.trim().is_empty()),
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
        created_at: response_string(&wire.created_at, "report.createdAt")?,
        output: ReportOutputData {
            template: response_string(&wire.output.template, "report.output.template")?,
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

pub(crate) fn parse_matching(payload: &[u8]) -> Result<MatchingAnalysisResponse> {
    let wire: MatchingWire = decode_json(payload)?;
    Ok(MatchingAnalysisResponse {
        id: response_string(&wire.id, "matching.id")?,
        created_at: response_string(&wire.created_at, "matching.createdAt")?,
        context: response_string(&wire.context, "matching.context")?,
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
        created_at: response_string(&wire.created_at, "media.createdAt")?,
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
        created_at: response_string(&wire.created_at, "media.createdAt")?,
        content_type: response_string(&wire.content_type, "media.contentType")?,
        label: response_string(&wire.label, "media.label")?,
        processing_status: response_string(&wire.processing_status, "media.processingStatus")?,
        last_used_at: wire.last_used_at.filter(|value| !value.trim().is_empty()),
        retention: MediaRetention {
            expires_at: wire
                .retention
                .expires_at
                .filter(|value| !value.trim().is_empty()),
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
        created_at: response_string(&wire.created_at, "entity.createdAt")?,
        label: wire.label.filter(|value| !value.trim().is_empty()),
        media_count: wire.media_count,
        last_seen_at: wire.last_seen_at.filter(|value| !value.trim().is_empty()),
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
        cursor: wire.cursor.filter(|value| !value.trim().is_empty()),
    })
}

pub(crate) fn parse_webhook_event(payload: &[u8]) -> Result<WebhookEvent> {
    let wire: WebhookEventWire = serde_json::from_slice(payload).map_err(|error| {
        ConduitError::invalid_webhook_payload("invalid webhook payload: invalid JSON")
            .with_source(error)
    })?;

    let event = WebhookEvent {
        id: response_string(&wire.id, "webhook.id")?,
        r#type: response_string(&wire.kind, "webhook.type")?,
        created_at: response_string(&wire.created_at, "webhook.createdAt")?,
        timestamp: response_string(&wire.timestamp, "webhook.timestamp")?,
        data: wire.data,
    };
    parse_iso8601(&event.created_at, "createdAt")?;
    parse_iso8601(&event.timestamp, "timestamp")?;
    Ok(event)
}

fn parse_matching_subject(wire: MatchingSubjectWire) -> MatchingResolvedSubject {
    MatchingResolvedSubject {
        source: wire.source,
        entity_id: wire.entity_id.filter(|value| !value.trim().is_empty()),
        resolved_label: wire.resolved_label.filter(|value| !value.trim().is_empty()),
    }
}

fn decode_json<T>(payload: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(payload)
        .map_err(|error| ConduitError::invalid_response("invalid JSON response").with_source(error))
}
