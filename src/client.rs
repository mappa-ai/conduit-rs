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

pub const DEFAULT_MAX_SOURCE_BYTES: u64 = 5 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: usize,
    pub max_source_bytes: u64,
    pub user_agent: Option<String>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            user_agent: None,
        }
    }
}

impl ClientOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = value.into();
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = value;
        self
    }

    pub fn max_retries(mut self, value: usize) -> Self {
        self.max_retries = value;
        self
    }

    pub fn max_source_bytes(mut self, value: u64) -> Self {
        self.max_source_bytes = value;
        self
    }

    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.user_agent = Some(value.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct Conduit {
    pub reports: ReportsResource,
    pub matching: MatchingResource,
    pub primitives: PrimitivesResource,
    pub webhooks: WebhooksResource,
}

impl Conduit {
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_options(api_key, ClientOptions::default())
    }

    pub fn with_options(api_key: impl Into<String>, options: ClientOptions) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(ConduitError::initialization(
                "api_key is required",
                "config_error",
            ));
        }
        if options.timeout.is_zero() {
            return Err(ConduitError::initialization(
                "timeout must be greater than zero",
                "config_error",
            ));
        }
        if options.max_source_bytes == 0 {
            return Err(ConduitError::initialization(
                "max_source_bytes must be greater than zero",
                "config_error",
            ));
        }

        let base_url = parse_base_url(&options.base_url)?;
        let transport = Arc::new(Transport::new(
            api_key.trim().to_string(),
            base_url,
            options.timeout,
            options.max_retries,
            options.user_agent.clone(),
        ));

        let jobs = JobsResource::new(transport.clone(), DEFAULT_POLL_INTERVAL);
        let media =
            MediaResource::new(transport.clone(), options.timeout, options.max_source_bytes);
        let entities = EntitiesResource::new(transport.clone());
        let reports = ReportsResource::new(transport.clone(), jobs.clone(), media.clone());
        let matching = MatchingResource::new(transport, jobs.clone());

        Ok(Self {
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
