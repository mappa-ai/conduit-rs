# Conduit Rust SDK

Official Rust SDK for the Conduit API.

## Install

```bash
cargo add conduit-rs
```

## Quickstart

```rust
use conduit_rs::{Conduit, CreateReportRequest, ReportOutput, ReportTemplate, Source, TargetSelector, Webhook};

#[tokio::main]
async fn main() -> Result<(), conduit_rs::ConduitError> {
    let client = Conduit::new("sk_...")?;

    let receipt = client
        .reports
        .create(CreateReportRequest {
            source: Source::path("./call.mp3"),
            output: ReportOutput::new(ReportTemplate::GeneralReport),
            target: TargetSelector::dominant(),
            webhook: Some(Webhook::new("https://my-app.com/webhooks/conduit")),
            idempotency_key: None,
            request_id: None,
        })
        .await?;

    println!("{}", receipt.job_id);

    Ok(())
}
```

Verify webhooks before parsing:

```rust
use conduit_rs::Conduit;
use http::HeaderMap;
use std::time::Duration;

fn handle_webhook(client: &Conduit, body: &[u8], headers: &HeaderMap, secret: &str) -> Result<(), conduit_rs::ConduitError> {
    client
        .webhooks
        .verify_signature(body, headers, secret, Duration::from_secs(300))?;

    let event = client.webhooks.parse_event(body)?;
    println!("{}", event.r#type);
    Ok(())
}
```

## Runtime notes

- Server-side Rust only.
- `Source::file`, `Source::url`, and `Source::path` currently materialize uploads in memory before submission.
- `handle.wait()` and `handle.stream()` use polling, not SSE.

## Local checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
