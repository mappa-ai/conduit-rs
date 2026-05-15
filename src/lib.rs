//! Official Rust SDK for the Conduit API.
//!
//! `Conduit` exposes five stable resource groups:
//!
//! - [`Conduit::reports`] for report generation and retrieval
//! - [`Conduit::psychometrics`] for direct psychometrics analysis and retrieval
//! - [`Conduit::matching`] for matching jobs and results
//! - [`Conduit::webhooks`] for webhook signature verification and event parsing
//! - [`Conduit::primitives`] for advanced access to media, jobs, and entities
//!
//! The primary production path is webhook-first: create a report or matching job, persist the
//! returned receipt, then react to a verified webhook when the job reaches a terminal state.
//! [`PsychometricsResource`] is the additional stable sync workflow for direct trait-map access.
//! [`ReportHandle`] and [`MatchingHandle`] provide polling helpers for scripts and local tools.

#![deny(missing_docs)]
#![deny(
    rustdoc::bare_urls,
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_rust_codeblocks
)]

mod client;
mod common;
mod error;
/// Matching workflow types and resource methods.
pub mod matching;
/// Typed response models returned by the SDK.
pub mod model;
/// Advanced low-level media, jobs, and entities resources.
pub mod primitives;
/// Psychometrics workflow types and resource methods.
pub mod psychometrics;
/// Report workflow types and resource methods.
pub mod reports;
mod transport;
/// Webhook verification and typed event parsing.
pub mod webhooks;

/// Top-level API client.
pub use client::{Conduit, ConduitBuilder, DEFAULT_MAX_SOURCE_BYTES};
/// Shared SDK error type and result alias.
pub use error::{ConduitError as Error, Result};
/// Matching workflow request, receipt, and handle types.
pub use matching::{
    MatchingContext, MatchingCreate, MatchingHandle, MatchingReceipt, MatchingResource, SubjectRef,
};
/// Common response models returned by Conduit resources.
pub use model::{
    CreditReservationStatus, Entity, FileDeleteReceipt, Job, JobCreditReservation, JobErrorData,
    JobEvent, JobEventKind, JobKind, JobStage, JobStatus, ListEntitiesResponse, ListFilesResponse,
    Matching, MatchingOutputData, MatchingResolvedSubject, MediaFile, MediaObject, MediaRetention,
    ReceiptStatus, Report, ReportOutputData, RetentionLockResult, Usage,
};
/// Advanced primitives and shared polling/source types.
pub use primitives::{
    ActionOptions, EntitiesResource, JobsResource, MediaResource, PrimitivesResource, Source,
    StreamOptions, WaitOptions,
};
/// Psychometrics workflow request and response types.
pub use psychometrics::{
    PsychometricsCreate, PsychometricsResource, PsychometricsResult, PsychometricsSource,
    PsychometricsTarget, PsychometricsTargetStrategy,
};
/// Report workflow request, receipt, and target configuration types.
pub use reports::{
    OnMiss, ReportCreate, ReportHandle, ReportReceipt, ReportTemplate, ReportsResource, Target,
    WebhookEndpoint,
};
/// Webhook event and verification types.
pub use webhooks::{
    MatchingCompletedEvent, MatchingFailedEvent, ReportCompletedEvent, ReportFailedEvent,
    UnknownWebhookEvent, WebhookEvent, WebhookFailure, WebhooksResource,
};
