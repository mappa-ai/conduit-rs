//! Psychometrics workflow types and resource methods.

use crate::common::{path_segment, require_non_empty};
use crate::error::{ConduitError, Result};
use crate::primitives::{MediaResource, Source};
use crate::transport::{MultipartFile, RequestBody, RequestOptions, Transport};
use reqwest::Method;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Stable target strategies supported by the psychometrics workflow.
pub enum PsychometricsTargetStrategy {
    /// Select the dominant speaker in the recording.
    Dominant,
    /// Select a speaker using a natural-language hint.
    MagicHint,
}

impl PsychometricsTargetStrategy {
    /// Returns the canonical API identifier for the target strategy.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dominant => "dominant",
            Self::MagicHint => "magic_hint",
        }
    }

    fn parse(value: &str, name: &str) -> Result<Self> {
        match value {
            "dominant" => Ok(Self::Dominant),
            "magic_hint" => Ok(Self::MagicHint),
            _ => Err(ConduitError::invalid_response(format!(
                "invalid {name}: expected dominant or magic_hint"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
/// Upload-capable source used by the psychometrics workflow.
pub enum PsychometricsSource {
    /// Upload in-memory bytes.
    File {
        /// File name reported to the API.
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

impl PsychometricsSource {
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

    /// Applies a user-facing label to the source.
    pub fn with_label(self, label: impl Into<String>) -> Self {
        let label = Some(label.into());
        match self {
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

    fn as_upload_source(&self) -> Source {
        match self {
            Self::File {
                file_name,
                data,
                label,
            } => {
                let mut source = Source::file(file_name.clone(), data.clone());
                if let Some(label) = label {
                    source = source.with_label(label.clone());
                }
                source
            }
            Self::Url { url, label } => {
                let mut source = Source::url(url.clone());
                if let Some(label) = label {
                    source = source.with_label(label.clone());
                }
                source
            }
            Self::Path { path, label } => {
                let mut source = Source::path(path.clone());
                if let Some(label) = label {
                    source = source.with_label(label.clone());
                }
                source
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Speaker selection strategy used when creating psychometrics.
pub enum PsychometricsTarget {
    /// Select the dominant speaker in the recording.
    Dominant,
    /// Select a speaker using a natural-language hint.
    Hint {
        /// Natural-language selection hint passed through to the API.
        hint: String,
    },
}

impl PsychometricsTarget {
    /// Creates a target that selects the dominant speaker.
    pub fn dominant() -> Self {
        Self::Dominant
    }

    /// Creates a target that selects a speaker using a natural-language hint.
    pub fn hint(hint: impl Into<String>) -> Self {
        Self::Hint { hint: hint.into() }
    }

    fn strategy(&self) -> PsychometricsTargetStrategy {
        match self {
            Self::Dominant => PsychometricsTargetStrategy::Dominant,
            Self::Hint { .. } => PsychometricsTargetStrategy::MagicHint,
        }
    }

    fn hint_value(&self) -> Result<Option<String>> {
        match self {
            Self::Dominant => Ok(None),
            Self::Hint { hint } => Ok(Some(require_non_empty(hint, "target.hint")?)),
        }
    }
}

#[derive(Debug, Clone)]
/// Request payload for [`PsychometricsResource::create`].
pub struct PsychometricsCreate {
    /// Input media source.
    pub source: PsychometricsSource,
    /// Speaker selection strategy.
    pub target: PsychometricsTarget,
    /// Optional idempotency key applied to the create request.
    pub idempotency_key: Option<String>,
    /// Optional request identifier echoed by the API.
    pub request_id: Option<String>,
}

impl PsychometricsCreate {
    /// Creates a new psychometrics request.
    pub fn new(source: PsychometricsSource, target: PsychometricsTarget) -> Self {
        Self {
            source,
            target,
            idempotency_key: None,
            request_id: None,
        }
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
/// Confidence metadata returned by the psychometrics endpoint.
pub struct PsychometricsConfidence {
    /// Overall confidence score between 0 and 1.
    pub overall: f64,
    /// Heuristic source used to compute confidence.
    pub source: String,
}

#[derive(Debug, Clone)]
/// Quality metadata returned by the psychometrics endpoint.
pub struct PsychometricsQuality {
    /// Number of scored target segments.
    pub segment_count: i64,
    /// Signal band: low, medium, or high.
    pub signal: String,
    /// Full source duration in seconds.
    pub source_audio_duration_seconds: f64,
    /// Proportion of source audio attributed to the selected speaker.
    pub speaker_coverage_ratio: f64,
    /// Scored target duration in seconds.
    pub target_audio_duration_seconds: f64,
    /// Number of scored target utterances.
    pub target_utterance_count: i64,
}

#[derive(Debug, Clone)]
/// Resolved speaker selection returned by the psychometrics endpoint.
pub struct PsychometricsSelectedSpeaker {
    /// Resolved zero-based speaker index.
    pub speaker_index: i64,
    /// Strategy that resolved the speaker.
    pub strategy: PsychometricsTargetStrategy,
}

#[derive(Debug, Clone)]
/// Model metadata returned by the psychometrics endpoint.
pub struct PsychometricsModelInfo {
    /// Backend model metadata.
    pub metadata: std::collections::HashMap<String, String>,
    /// Backend model version when reported.
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
/// Completed psychometrics analysis.
pub struct PsychometricsResult {
    /// Stable analysis identifier.
    pub analysis_id: String,
    /// Confidence metadata.
    pub confidence: PsychometricsConfidence,
    /// Analysis creation timestamp.
    pub created_at: OffsetDateTime,
    /// Analysis expiry timestamp.
    pub expires_at: OffsetDateTime,
    /// Model metadata.
    pub model: PsychometricsModelInfo,
    /// Trait scores keyed by stable trait identifier.
    pub psychometrics: std::collections::HashMap<String, f64>,
    /// Quality metadata.
    pub quality: PsychometricsQuality,
    /// Resolved speaker selection.
    pub selected_speaker: PsychometricsSelectedSpeaker,
}

#[derive(Debug, Clone)]
/// Psychometrics API resource group.
pub struct PsychometricsResource {
    media: MediaResource,
    transport: Arc<Transport>,
}

impl PsychometricsResource {
    pub(crate) fn new(transport: Arc<Transport>, media: MediaResource) -> Self {
        Self { media, transport }
    }

    /// Creates a psychometrics analysis and returns the completed result.
    pub async fn create(&self, request: PsychometricsCreate) -> Result<PsychometricsResult> {
        let hint = request.target.hint_value()?;
        let materialized = self
            .media
            .materialize_source(request.source.as_upload_source())
            .await?;
        let mut fields = vec![(
            "strategy".to_string(),
            request.target.strategy().as_str().to_string(),
        )];
        if let Some(hint) = hint {
            fields.push(("hint".to_string(), hint));
        }
        let response = self
            .transport
            .request(
                Method::POST,
                "/v2/psychometrics",
                RequestOptions {
                    body: RequestBody::Multipart {
                        fields,
                        file: MultipartFile {
                            file_name: materialized.file_name,
                            content_type: materialized.content_type,
                            payload: materialized.payload,
                        },
                    },
                    idempotency_key: request.idempotency_key,
                    request_id: request.request_id,
                    retryable: false,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_psychometrics_result(&response.body)
    }

    /// Fetches a previously completed psychometrics analysis by ID.
    pub async fn get(&self, analysis_id: &str) -> Result<PsychometricsResult> {
        let response = self
            .transport
            .request(
                Method::GET,
                &format!(
                    "/v2/psychometrics/{}",
                    path_segment(analysis_id, "analysis_id")?
                ),
                RequestOptions {
                    retryable: true,
                    ..RequestOptions::default()
                },
            )
            .await?;
        parse_psychometrics_result(&response.body)
    }
}

#[derive(Debug, Deserialize)]
struct PsychometricsResultWire {
    #[serde(rename = "analysisId")]
    analysis_id: String,
    confidence: PsychometricsConfidenceWire,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "expiresAt")]
    expires_at: String,
    model: PsychometricsModelInfoWire,
    psychometrics: std::collections::HashMap<String, f64>,
    quality: PsychometricsQualityWire,
    #[serde(rename = "selectedSpeaker")]
    selected_speaker: PsychometricsSelectedSpeakerWire,
}

#[derive(Debug, Deserialize)]
struct PsychometricsConfidenceWire {
    overall: f64,
    source: String,
}

#[derive(Debug, Deserialize)]
struct PsychometricsQualityWire {
    #[serde(rename = "segmentCount")]
    segment_count: i64,
    signal: String,
    #[serde(rename = "sourceAudioDurationSeconds")]
    source_audio_duration_seconds: f64,
    #[serde(rename = "speakerCoverageRatio")]
    speaker_coverage_ratio: f64,
    #[serde(rename = "targetAudioDurationSeconds")]
    target_audio_duration_seconds: f64,
    #[serde(rename = "targetUtteranceCount")]
    target_utterance_count: i64,
}

#[derive(Debug, Deserialize)]
struct PsychometricsSelectedSpeakerWire {
    #[serde(rename = "speakerIndex")]
    speaker_index: i64,
    strategy: String,
}

#[derive(Debug, Deserialize)]
struct PsychometricsModelInfoWire {
    metadata: std::collections::HashMap<String, String>,
    version: Option<String>,
}

fn parse_psychometrics_result(payload: &[u8]) -> Result<PsychometricsResult> {
    let wire: PsychometricsResultWire = decode_json(payload)?;
    let confidence_source =
        response_string(&wire.confidence.source, "psychometrics.confidence.source")?;
    if confidence_source != "signal_heuristic" {
        return Err(ConduitError::invalid_response(
            "invalid psychometrics.confidence.source: expected signal_heuristic",
        ));
    }
    let quality_signal = response_string(&wire.quality.signal, "psychometrics.quality.signal")?;
    validate_quality_signal(&quality_signal, "psychometrics.quality.signal")?;
    Ok(PsychometricsResult {
        analysis_id: response_string(&wire.analysis_id, "psychometrics.analysisId")?,
        confidence: PsychometricsConfidence {
            overall: wire.confidence.overall,
            source: confidence_source,
        },
        created_at: response_datetime(&wire.created_at, "psychometrics.createdAt")?,
        expires_at: response_datetime(&wire.expires_at, "psychometrics.expiresAt")?,
        model: PsychometricsModelInfo {
            metadata: wire.model.metadata,
            version: wire.model.version.filter(|value| !value.trim().is_empty()),
        },
        psychometrics: wire.psychometrics,
        quality: PsychometricsQuality {
            segment_count: wire.quality.segment_count,
            signal: quality_signal,
            source_audio_duration_seconds: wire.quality.source_audio_duration_seconds,
            speaker_coverage_ratio: wire.quality.speaker_coverage_ratio,
            target_audio_duration_seconds: wire.quality.target_audio_duration_seconds,
            target_utterance_count: wire.quality.target_utterance_count,
        },
        selected_speaker: PsychometricsSelectedSpeaker {
            speaker_index: wire.selected_speaker.speaker_index,
            strategy: PsychometricsTargetStrategy::parse(
                &wire.selected_speaker.strategy,
                "psychometrics.selectedSpeaker.strategy",
            )?,
        },
    })
}

fn decode_json<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T> {
    serde_json::from_slice(payload)
        .map_err(|error| ConduitError::invalid_response("invalid JSON response").with_source(error))
}

fn response_datetime(value: &str, name: &str) -> Result<OffsetDateTime> {
    let normalized = response_string(value, name)?;
    OffsetDateTime::parse(&normalized, &Rfc3339).map_err(|error| {
        ConduitError::invalid_response(format!("invalid {name}: expected RFC3339 timestamp"))
            .with_source(error)
    })
}

fn response_string(value: &str, name: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(ConduitError::invalid_response(format!(
            "invalid {name}: expected non-empty string"
        )));
    }
    Ok(value.to_string())
}

fn validate_quality_signal(value: &str, name: &str) -> Result<()> {
    match value {
        "low" | "medium" | "high" => Ok(()),
        _ => Err(ConduitError::invalid_response(format!(
            "invalid {name}: expected low, medium, or high"
        ))),
    }
}
