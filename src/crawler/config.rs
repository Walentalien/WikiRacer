use std::sync::Arc;
use url::Url;

/// Default timeout for link requests in seconds
pub const LINK_REQUEST_TIMEOUT_SEC: u64 = 2;

/// Configuration for the crawler
pub struct CrawlerConfig {
    pub starting_url: Url,
    pub scraping_foreign_hosts: bool,
    pub max_urls: usize,
    pub max_depth: usize,
    pub thread_count: usize,
    pub request_delay_ms: u64,
    pub max_retries: usize,
    pub request_timeout_sec: u64,
    pub target_url: Option<Url>,
}

impl CrawlerConfig {
    pub fn new(starting_url: Url) -> Self {
        Self {
            starting_url,
            scraping_foreign_hosts: false,
            max_urls: 10,
            max_depth: 3,
            thread_count: 2,
            request_delay_ms: 100,
            max_retries: 3,
            request_timeout_sec: LINK_REQUEST_TIMEOUT_SEC,
            target_url: None,
        }
    }

    pub fn with_max_urls(mut self, max_urls: usize) -> Self {
        self.max_urls = max_urls;
        self
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = thread_count;
        self
    }

    pub fn with_foreign_hosts(mut self, allow: bool) -> Self {
        self.scraping_foreign_hosts = allow;
        self
    }

    pub fn with_request_delay(mut self, delay_ms: u64) -> Self {
        self.request_delay_ms = delay_ms;
        self
    }

    pub fn with_target_url(mut self, url: Url) -> Self {
        self.target_url = Some(url);
        self
    }
}

pub type CrawlerConfigRef = Arc<CrawlerConfig>;