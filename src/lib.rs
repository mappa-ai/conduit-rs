#![allow(clippy::result_large_err)]

mod client;
mod common;
mod error;
mod matching;
mod model;
mod primitives;
mod reports;
mod transport;
mod webhooks;

pub use client::{ClientOptions, Conduit, DEFAULT_MAX_SOURCE_BYTES};
pub use error::ConduitError;
pub use matching::{
    CreateMatchingRequest, MatchingContext, MatchingJobReceipt, MatchingResource,
    MatchingRunHandle, MatchingSubject,
};
pub use model::{
    Entity, FileDeleteReceipt, Job, JobCreditReservation, JobErrorData, JobEvent,
    ListEntitiesResponse, ListFilesResponse, MatchingAnalysisResponse, MatchingOutputData,
    MatchingResolvedSubject, MediaFile, MediaObject, MediaRetention, Report, ReportOutputData,
    RetentionLockResult, Usage, WebhookEvent,
};
pub use primitives::{
    EntitiesResource, JobsResource, MediaResource, PrimitivesResource, Source, StreamOptions,
    WaitOptions,
};
pub use reports::{
    CreateReportRequest, OnMiss, ReportJobReceipt, ReportOutput, ReportRunHandle, ReportTemplate,
    ReportsResource, TargetSelector, Webhook,
};
pub use webhooks::WebhooksResource;
