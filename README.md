# Conduit Rust SDK

[![crates.io](https://img.shields.io/crates/v/conduit-rs.svg)](https://crates.io/crates/conduit-rs)
[![docs.rs](https://docs.rs/conduit-rs/badge.svg)](https://docs.rs/conduit-rs)
[![GitHub](https://img.shields.io/badge/github-mappa--ai%2Fconduit--rs-black)](https://github.com/mappa-ai/conduit-rs)

This repository is the public mirror, documentation surface, and issue tracker for the Conduit Rust SDK.

Conduit is a behavioral intelligence API with two primary workflows:

- `reports` turns a recording into a structured behavioral analysis for one selected speaker.
- `matching` evaluates one target subject against a group in a closed interpretation context.

## Install

```bash
cargo add conduit-rs
```

## Runtime Support

This SDK targets server-side Rust runtimes.

| Capability | Rust server runtime |
| --- | --- |
| `Source::file(...)` | Supported |
| `Source::url(...)` | Supported |
| `Source::path(...)` | Supported |
| `receipt.handle.wait()` | Supported |
| `receipt.handle.stream()` | Supported |
| `webhooks.verify_signature()` | Supported |

Notes:

- The SDK is designed for server-side use with a secret API key.
- `Source::file`, `Source::url`, and `Source::path` currently materialize uploads in memory before submission.
- `Source::url(...)` performs an SDK-side fetch, then uploads the fetched bytes to Conduit.
- `handle.wait()` and `handle.stream()` are polling helpers for local tools and scripts. Webhooks are the recommended production completion path.

## Mental Model

The client is intentionally small:

- `conduit.reports()` is the main onboarding surface.
- `conduit.matching()` is the stable matching workflow.
- `conduit.webhooks()` verifies and parses webhook deliveries.
- `conduit.primitives()` exposes advanced low-level resources for `media`, `jobs`, and `entities`.

Create calls return a receipt immediately after any upload step completes and the job is accepted. Use:

- webhooks in production
- `receipt.handle.wait()` in local development or scripts
- `receipt.handle.stream()` when you want status and stage events while polling

## Quickstart: Reports With Webhooks

```rust
use conduit_rs::{
    Conduit, ReportCreate, ReportTemplate, Source, Target, WebhookEndpoint, WebhookEvent,
};
use http::HeaderMap;

#[tokio::main]
async fn main() -> Result<(), conduit_rs::Error> {
    let conduit = Conduit::builder("sk_...")
        .max_retries(2)
        .build()?;

    let receipt = conduit
        .reports()
        .create(
            ReportCreate::new(
                Source::url("https://storage.example.com/call.wav"),
                ReportTemplate::GeneralReport,
                Target::dominant(),
            )
            .webhook(WebhookEndpoint::new("https://my-app.com/webhooks/conduit"))
            .idempotency_key("signup-call-42"),
        )
        .await?;

    println!("queued report job: {}", receipt.job_id);
    Ok(())
}

async fn handle_webhook(
    conduit: &Conduit,
    body: &[u8],
    headers: &HeaderMap,
    secret: &str,
) -> Result<(), conduit_rs::Error> {
    conduit.webhooks().verify_signature(body, headers, secret)?;

    match conduit.webhooks().parse_event(body)? {
        WebhookEvent::ReportCompleted(event) => {
            let report = conduit.reports().get(&event.report_id).await?;
            println!("report ready: {}", report.id);
        }
        WebhookEvent::ReportFailed(event) => {
            eprintln!("job {} failed: {}", event.job_id, event.error.message);
        }
        WebhookEvent::Unknown(event) => {
            println!("ignoring unknown webhook event: {}", event.event_type);
        }
        _ => {}
    }

    Ok(())
}
```

## Quickstart: Matching

```rust
use conduit_rs::{Conduit, MatchingContext, MatchingCreate, SubjectRef};

async fn create_matching(conduit: &Conduit) -> Result<(), conduit_rs::Error> {
    let receipt = conduit
        .matching()
        .create(MatchingCreate::new(
            MatchingContext::BehavioralCompatibility,
            SubjectRef::entity("ent_candidate"),
            vec![
                SubjectRef::entity("ent_manager"),
                SubjectRef::entity("ent_peer"),
            ],
        ))
        .await?;

    println!("queued matching job: {}", receipt.job_id);
    Ok(())
}
```

## Local Development: Wait For Completion

`wait()` and `stream()` are convenience helpers for scripts and local development. They are not the default production integration path.

```rust
use conduit_rs::{Conduit, ReportCreate, ReportTemplate, Source, Target};
use std::time::Duration;

async fn wait_for_report(conduit: &Conduit) -> Result<(), conduit_rs::Error> {
    let receipt = conduit
        .reports()
        .create(ReportCreate::new(
            Source::path("./call.mp3"),
            ReportTemplate::GeneralReport,
            Target::dominant(),
        ))
        .await?;

    let report = receipt.handle.wait_for(Duration::from_secs(300)).await?;
    println!("report ready: {}", report.id);
    Ok(())
}
```

## Core Types

- `Source` chooses where input media comes from: an existing `media_id`, in-memory bytes, a remote URL, or a filesystem path.
- `Target` selects which speaker should be analyzed in a report.
- `SubjectRef` references either a stable entity or a media target for matching.
- `WebhookEndpoint` configures the completion webhook attached to `create(...)` calls.
- `WaitOptions` and `StreamOptions` customize polling helpers.

## Primitives

`conduit.primitives()` exposes the advanced stable surface:

- `primitives.media` uploads and manages source media
- `primitives.jobs` inspects and cancels long-running jobs
- `primitives.entities` reads and updates stable speaker identities

Use primitives when you need lower-level control. For onboarding, start with `reports()`.

## Webhooks

Webhook handling should always follow this order:

1. Read the exact raw request body.
2. Call `verify_signature(...)` before parsing anything.
3. Call `parse_event(...)` to get a typed `WebhookEvent`.
4. Handle known events such as `ReportCompleted` or `MatchingFailed`.

Signature verification expects the `conduit-signature` header and validates the delivery with HMAC-SHA256.

## Errors

The crate returns `conduit_rs::Error` for all SDK failures.

Useful consumer-facing helpers:

- `error.code()` returns a stable SDK or API error code
- `error.request_id()` returns the request identifier when the API provided one
- `error.status()` returns the HTTP status code for API-derived failures

Typical categories include:

- initialization and configuration failures
- source validation and remote fetch failures
- typed API failures such as auth, validation, rate limits, and insufficient credits
- webhook verification failures
- timeout, cancellation, and stream errors

## Behavior Notes And Limits

- `reports().create(...)` never waits for analysis completion.
- `matching().create(...)` never waits for matching completion.
- `Source::url(...)` follows redirects up to the server-side limit configured by the SDK.
- `Source::path(...)` resolves relative paths from the current working directory.
- `ReportHandle::report()` and `MatchingHandle::matching()` return `Ok(None)` until the job has completed successfully.

## Local Checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Issues

- Report SDK issues: `https://github.com/mappa-ai/conduit-rs/issues`
