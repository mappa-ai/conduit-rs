use conduit_rs::Conduit;
use hmac::{Hmac, Mac};
use http::HeaderMap;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

fn signed_header(payload: &[u8], secret: &str, timestamp: i64) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("mac");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    )
}

#[test]
fn verify_signature_accepts_valid_header() {
    let client = Conduit::new("sk_test").expect("client");
    let payload = br#"{"id":"evt_1","type":"report.completed","createdAt":"2026-03-15T12:00:00Z","timestamp":"2026-03-15T12:00:00Z","data":{"jobId":"job_1","reportId":"rep_1","status":"succeeded"}}"#;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs() as i64;
    let mut headers = HeaderMap::new();
    headers.insert(
        "conduit-signature",
        signed_header(payload, "whsec_test", timestamp)
            .parse()
            .expect("header"),
    );

    client
        .webhooks()
        .verify_signature(payload, &headers, "whsec_test")
        .expect("signature should verify");
}

#[test]
fn verify_signature_rejects_missing_header() {
    let client = Conduit::new("sk_test").expect("client");
    let error = client
        .webhooks()
        .verify_signature(b"{}", &HeaderMap::new(), "whsec_test")
        .expect_err("missing header should fail");

    assert_eq!(error.code(), "webhook_signature_missing");
}

#[test]
fn parse_event_validates_known_payloads() {
    let client = Conduit::new("sk_test").expect("client");
    let payload = br#"{"id":"evt_1","type":"matching.failed","createdAt":"2026-03-15T12:00:00Z","timestamp":"2026-03-15T12:00:00Z","data":{"jobId":"job_1","status":"failed","error":{"code":"job_failed","message":"boom"}}}"#;

    let event = client
        .webhooks()
        .parse_event(payload)
        .expect("event should parse");
    match event {
        conduit_rs::WebhookEvent::MatchingFailed(event) => {
            assert_eq!(event.job_id, "job_1");
            assert_eq!(event.error.code, "job_failed");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
