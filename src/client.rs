use crate::common::parse_base_url;
use crate::error::{ConduitError, Result};
use crate::matching::MatchingResource;
use crate::primitives::{EntitiesResource, JobsResource, MediaResource, PrimitivesResource};
use crate::reports::ReportsResource;
use crate::transport::Transport;
use crate::webhooks::WebhooksResource;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.mappa.ai";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(3);
const DEFAULT_MAX_RETRIES: usize = 2;

/// Default maximum source size, in bytes, accepted by SDK-managed uploads.
pub const DEFAULT_MAX_SOURCE_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// Builder used to configure and construct a [`Conduit`] client.
///
/// Most consumers only need an API key:
///
/// ```rust,no_run
/// use conduit_rs::Conduit;
///
/// let conduit = Conduit::builder("sk_test").build();
/// # let _ = conduit;
/// ```
#[derive(Debug, Clone)]
pub struct ConduitBuilder {
    api_key: String,
    base_url: String,
    timeout: Duration,
    max_retries: usize,
    max_source_bytes: u64,
    user_agent: Option<String>,
}

impl ConduitBuilder {
    /// Creates a new builder for the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            user_agent: None,
        }
    }

    /// Overrides the API base URL.
    ///
    /// This is primarily useful for development, testing, or self-hosted environments.
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = value.into();
        self
    }

    /// Sets the per-request timeout used by SDK-managed network operations.
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    /// Sets the retry budget for transient API failures.
    pub fn max_retries(mut self, value: usize) -> Self {
        self.max_retries = value;
        self
    }

    /// Sets the maximum number of bytes accepted for SDK-managed uploads.
    pub fn max_source_bytes(mut self, value: u64) -> Self {
        self.max_source_bytes = value;
        self
    }

    /// Appends a custom user agent fragment to SDK requests.
    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.user_agent = Some(value.into());
        self
    }

    /// Builds the configured [`Conduit`] client.
    pub fn build(self) -> Result<Conduit> {
        let api_key = self.api_key.trim();
        if api_key.is_empty() {
            return Err(ConduitError::initialization(
                "api_key is required",
                "config_error",
            ));
        }
        if self.timeout.is_zero() {
            return Err(ConduitError::initialization(
                "timeout must be greater than zero",
                "config_error",
            ));
        }
        if self.max_source_bytes == 0 {
            return Err(ConduitError::initialization(
                "max_source_bytes must be greater than zero",
                "config_error",
            ));
        }

        let base_url = parse_base_url(&self.base_url)?;
        let transport = Arc::new(Transport::new(
            api_key.to_string(),
            base_url,
            self.timeout,
            self.max_retries,
            self.user_agent,
        ));

        let jobs = JobsResource::new(transport.clone(), DEFAULT_POLL_INTERVAL);
        let media = MediaResource::new(transport.clone(), self.timeout, self.max_source_bytes);
        let entities = EntitiesResource::new(transport.clone());
        let reports = ReportsResource::new(transport.clone(), jobs.clone(), media.clone());
        let matching = MatchingResource::new(transport, jobs.clone());

        Ok(Conduit {
            reports,
            matching,
            primitives: PrimitivesResource {
                entities,
                media,
                jobs,
            },
            webhooks: WebhooksResource,
        })
    }
}

impl Default for ConduitBuilder {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Main entry point for the Conduit API.
///
/// The client is intentionally small and organized by resource group. Start with [`Self::reports`]
/// for report generation, then branch into matching, webhooks, or primitives as needed.
#[derive(Debug, Clone)]
pub struct Conduit {
    reports: ReportsResource,
    matching: MatchingResource,
    primitives: PrimitivesResource,
    webhooks: WebhooksResource,
}

impl Conduit {
    /// Creates a client with default configuration.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::builder(api_key).build()
    }

    /// Creates a builder for a new client.
    pub fn builder(api_key: impl Into<String>) -> ConduitBuilder {
        ConduitBuilder::new(api_key)
    }

    /// Returns the reports resource group.
    ///
    /// This is the primary onboarding surface for the SDK.
    pub fn reports(&self) -> &ReportsResource {
        &self.reports
    }

    /// Returns the matching resource group.
    pub fn matching(&self) -> &MatchingResource {
        &self.matching
    }

    /// Returns the advanced low-level primitives surface.
    pub fn primitives(&self) -> &PrimitivesResource {
        &self.primitives
    }

    /// Returns the webhook verification and parsing helpers.
    pub fn webhooks(&self) -> &WebhooksResource {
        &self.webhooks
    }
}
