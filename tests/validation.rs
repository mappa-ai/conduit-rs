use conduit_rs::{
    ClientOptions, Conduit, CreateMatchingRequest, CreateReportRequest, MatchingContext,
    MatchingSubject, ReportOutput, ReportTemplate, Source, TargetSelector,
};
use std::time::Duration;

fn test_client() -> Conduit {
    Conduit::with_options(
        "sk_test",
        ClientOptions::new()
            .base_url("http://127.0.0.1:9999")
            .timeout(Duration::from_secs(1)),
    )
    .expect("client")
}

#[test]
fn missing_api_key_fails() {
    let error = Conduit::new("").expect_err("missing api key should fail");
    assert_eq!(error.code(), "config_error");
}

#[tokio::test]
async fn report_create_rejects_invalid_timerange_before_network() {
    let client = test_client();
    let error = client
        .reports
        .create(CreateReportRequest {
            source: Source::media_id("med_123"),
            output: ReportOutput::new(ReportTemplate::GeneralReport),
            target: TargetSelector::time_range(None, None),
            webhook: None,
            idempotency_key: None,
            request_id: None,
        })
        .await
        .expect_err("invalid timerange should fail");

    assert_eq!(error.code(), "invalid_request");
    assert!(error.to_string().contains("target.timeRange must include"));
}

#[tokio::test]
async fn matching_create_rejects_duplicate_entity_ids_before_network() {
    let client = test_client();
    let error = client
        .matching
        .create(CreateMatchingRequest {
            context: MatchingContext::HiringTeamFit,
            target: MatchingSubject::entity_id("ent_1"),
            group: vec![MatchingSubject::entity_id("ent_1")],
            webhook: None,
            idempotency_key: None,
            request_id: None,
        })
        .await
        .expect_err("duplicate entity ids should fail");

    assert_eq!(error.code(), "invalid_request");
    assert!(error.to_string().contains("different direct entity IDs"));
}

#[tokio::test]
async fn media_upload_rejects_media_id_source_before_network() {
    let client = test_client();
    let error = client
        .primitives
        .media
        .upload(Source::media_id("med_123"), None, None)
        .await
        .expect_err("mediaId upload source should fail");

    assert_eq!(error.code(), "invalid_source");
}
