use std::collections::{VecDeque, HashMap};
use std::fs;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicUsize;
use log2::{debug, info, trace};
use url::Url;
use reqwest::Client;
use anyhow::{Result, anyhow};
use scraper::{Html, Selector};
use serde::Serialize;

const LINK_REQUEST_TIMEOUT_SEC: u64 = 2;

/// Configuration of current state of the crawler
pub struct CrawlerState {
    pub links_crawled_count: AtomicUsize,
    pub link_to_crawl_queue: RwLock<VecDeque<(Url, usize)>>, // (url, depth)
    pub links_graph: RwLock<Vec<Link>>,
    pub visited_urls: RwLock<std::collections::HashSet<Url>>,
    pub url_relationships: RwLock<HashMap<Url, Vec<Url>>>, // parent -> children
}
pub type CrawlerStateRef = Arc<CrawlerState>;

pub struct CrawlerConfig {
    /// starting point of scraping
    pub starting_url: Url,
    /// if should scrape foreign hosts
    pub scraping_foreign_hosts: bool,
    /// max urls to scrape before stopping
    pub max_urls: usize,
    /// maximum depth to crawl (0 = only starting page, 1 = starting page + links from it, etc.)
    pub max_depth: usize,
    /// number of worker threads for concurrent crawling
    pub thread_count: usize,
    /// delay between requests in milliseconds (to be respectful to servers)
    pub request_delay_ms: u64,
    /// maximum number of retries for failed requests
    pub max_retries: usize,
    /// timeout for each request in seconds
    pub request_timeout_sec: u64,
}

impl CrawlerConfig {
    pub fn new(starting_url: Url) -> Self {
        Self {
            starting_url,
            scraping_foreign_hosts: false,
            max_urls: 10000,
            max_depth: 30,
            thread_count: 10,
            request_delay_ms: 2000,
            max_retries: 3,
            request_timeout_sec: LINK_REQUEST_TIMEOUT_SEC,
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
}

impl CrawlerState {
    pub fn new(starting_url: Url) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((starting_url, 0)); // Start at depth 0

        Self {
            links_crawled_count: AtomicUsize::new(0),
            link_to_crawl_queue: RwLock::new(queue),
            links_graph: RwLock::new(Vec::new()),
            visited_urls: RwLock::new(std::collections::HashSet::new()),
            url_relationships: RwLock::new(HashMap::new()),
        }
    }
}

type CrawlerConfigRef = Arc<CrawlerConfig>;




#[derive(Serialize, Clone)]
pub struct Link {
    pub id: LinkID,
    pub url: String,
    pub depth: usize,
    pub children: Vec<LinkID>,
    pub parents: Vec<LinkID>,
}

pub type LinkID = usize;
// Arc for simultaneous access by multiple threads




/// If `path` is full_url -> returns it
/// if relative constructs full url  by merging with `root_url`
/// If there is trailing slash it removes it
fn construct_url(path: &str, root_url: Url) -> Result<Url, url::ParseError> {
    let mut url = if let Ok(parsed_url) = Url::parse(path) {
        if parsed_url.host().is_some() {
            parsed_url
        } else {
            root_url.join(path)?
        }
    } else {
        root_url.join(path)?
    };

    // Normalize: trim trailing slashes from path and remove fragments
    let trimmed_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&trimmed_path);

    // Remove fragment for consistent URLs
    url.set_fragment(None);

    trace!("Constructed link URL: {}", url);
    Ok(url)

}

/// Reads one link from `link_to_crawl_queue` and scrapes all links from there to the end of that queue
async fn scrape_page(url: Url, client: &Client, config: &CrawlerConfig) -> Result<Vec<Url>> {
    trace!("Scraping page: {}", url);

    let response = client
        .get(url.clone())
        // .timeout(std::time::Duration::from_secs(LINK_REQUEST_TIMEOUT_SEC))
        .timeout(std::time::Duration::from_secs(config.request_timeout_sec))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch page: {}", response.status()));
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").map_err(|e| anyhow!("Failed to parse selector: {}", e))?;

    let mut found_urls = Vec::new();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(parsed_url) = construct_url(href, url.clone()) {


                // Check if we should include this URL
                let should_include = if config.scraping_foreign_hosts {
                    true
                } else {
                    // For Wikipedia, allow different language versions (en.wikipedia.org, es.wikipedia.org, etc.)
                    if let (Some(parsed_host), Some(root_host)) = (parsed_url.host_str(), url.host_str()) {
                        parsed_host == root_host ||
                            (parsed_host.ends_with(".wikipedia.org") && root_host.ends_with(".wikipedia.org"))
                    } else {
                        parsed_url.host() == url.host()
                    }
                };

                if should_include {
                    found_urls.push(parsed_url);
                } else {
                    debug!("Skipped scraping {parsed_url} because its foreign host");
                }
            }
        }
    }

    trace!("Found {} urls on page {}", found_urls.len(), url);
    Ok(found_urls)
}


pub async fn crawl(crawler_state_ref: CrawlerStateRef, crawler_cfg_ref: CrawlerConfigRef) -> Result<()> {
    use tokio::time::{sleep, Duration};
    use std::sync::atomic::{AtomicBool, Ordering};

    let should_stop = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    // Spawn worker threads
    for worker_id in 0..crawler_cfg_ref.thread_count {
        let state = Arc::clone(&crawler_state_ref);
        let config = Arc::clone(&crawler_cfg_ref);
        let stop_flag = Arc::clone(&should_stop);

        let handle = tokio::spawn(async move {
            let client = Client::new();
            info!("Worker {} started", worker_id);

            while !stop_flag.load(Ordering::Relaxed) {
                // Get next URL to crawl
                let next_item = {
                    let mut queue = state.link_to_crawl_queue.write().unwrap();
                    queue.pop_front()
                };

                if let Some((url, depth)) = next_item {
                    // Check if we've exceeded max depth
                    if depth >= config.max_depth {
                        debug!("Skipping {} - max depth {} reached", url, config.max_depth);
                        continue;
                    }

                    // Check if we've already visited this URL
                    {
                        let mut visited = state.visited_urls.write().unwrap();
                        if visited.contains(&url) {
                            continue;
                        }
                        visited.insert(url.clone());
                    }

                    // Check if we've reached max URLs
                    let current_count = state.links_crawled_count.load(Ordering::Relaxed);
                    if current_count >= config.max_urls {
                        info!("Worker {}: Max URLs reached", worker_id);
                        stop_flag.store(true, Ordering::Relaxed);
                        break;
                    }

                    info!("Worker {}: Crawling {} at depth {}", worker_id, url, depth);

                    // Scrape the page
                    match scrape_page(url.clone(), &client, &config).await {
                        Ok(found_urls) => {
                            // Store relationships
                            {
                                let mut relationships = state.url_relationships.write().unwrap();
                                relationships.insert(url.clone(), found_urls.clone());
                            }

                            // Add found URLs to queue for next depth level
                            {
                                let mut queue = state.link_to_crawl_queue.write().unwrap();
                                for found_url in found_urls {
                                    queue.push_back((found_url, depth + 1));
                                }
                            }

                            state.links_crawled_count.fetch_add(1, Ordering::Relaxed);

                            // Add delay to be respectful to servers
                            if config.request_delay_ms > 0 {
                                sleep(Duration::from_millis(config.request_delay_ms)).await;
                            }
                        }
                        Err(e) => {
                            debug!("Worker {}: Failed to scrape {}: {}", worker_id, url, e);
                        }
                    }
                } else {
                    // No more URLs to crawl, wait a bit or exit
                    sleep(Duration::from_millis(100)).await;

                    // Check if all workers are idle (no more work)
                    let queue_empty = {
                        let queue = state.link_to_crawl_queue.read().unwrap();
                        queue.is_empty()
                    };

                    if queue_empty {
                        info!("Worker {}: No more URLs to crawl", worker_id);
                        stop_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }

            info!("Worker {} finished", worker_id);
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.await.map_err(|e| anyhow!("Worker thread panicked: {}", e))?;
    }

    info!("Crawling completed. Total URLs processed: {}", crawler_state_ref.links_crawled_count.load(Ordering::Relaxed));
    Ok(())
}

/// Build a graph from the crawled state for pathfinding
pub fn build_graph_from_state(state: &CrawlerStateRef) -> HashMap<Url, Vec<Url>> {
    let relationships = state.url_relationships.read().unwrap();
    relationships.clone()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use url::Url;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    use crate::crawler::{construct_url, scrape_page, CrawlerConfig, LINK_REQUEST_TIMEOUT_SEC};

    // tests for construct_url start here
    #[test]
    fn test_get_url() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://google.com")?;
        let another_full_url = Url::parse("https://anotherlink.com");
        let result_url = construct_url(another_full_url.clone().unwrap().as_str(), root_url)?;
        assert_eq!(result_url, another_full_url.unwrap());
        Ok(())
    }

    #[test]
    fn test_get_url_with_relative_path() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        // for the path arg we have relative path
        //let relative_path = Url::parse("/some/relative/path")?;
        let relative_path =  "/some/relative/path";
        let result_url = construct_url(relative_path, root_url )?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }

    #[test]
    fn test_malformed_url() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://valid.root.url")?;
        let malformed_url = "this_is_a_malformed_url";
        let result_url = construct_url(malformed_url, root_url);
        assert_eq!("https://valid.root.url/this_is_a_malformed_url", result_url.unwrap().to_string());
        Ok(())
    }

    #[test]
    fn test_empty_path() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let result_url = construct_url("", root_url)?;
        let expected_url = Url::parse("https://groq.xyz/")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }
    #[test]
    fn test_url_with_query() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let relative_path = "/some/relative/path?query=1";
        let result_url = construct_url(relative_path, root_url)?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path?query=1")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }
    #[test]
    fn test_hash_fragment() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let relative_path = "/some/relative/path#section/";
        let result_url = construct_url(relative_path, root_url)?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path#section")?;
        assert_eq!(result_url, expected_url);
        Ok(())
    }
    #[test]
    fn test_get_url_with_link_with_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;

        let relative_path_wo_trailing_slash = "/some/relative/path";
        let relative_path_w_trailing_slash = "/some/relative/path/";

        let result_wo_trailing_slash = construct_url(relative_path_wo_trailing_slash, root_url.clone())?;
        let result_w_trailing_slash = construct_url(relative_path_w_trailing_slash, root_url)?;

        assert_eq!(result_wo_trailing_slash, result_w_trailing_slash);
        Ok(())
    }
    // tests for construct_url end here

    // tests for `scrape page` start here

    #[tokio::test]
    async fn test_scrape_empty_page() -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        //let url = Url::parse("https://example.com/empty")?;
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0, max_depth: 0, thread_count: 0, request_delay_ms: 0, max_retries: 0, request_timeout_sec: 0 };


        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/empty"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&mock_server)
            .await;
        let url = Url::parse(&format!("{}/empty", &mock_server.uri()))?;
        let result = scrape_page(url, &client, &config).await;
        assert_eq!(result?, Vec::<Url>::new());
        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_with_links() -> Result<(), Box<dyn std::error::Error>> {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use url::Url;

        let client = reqwest::Client::new();
        let mock_server = MockServer::start().await;
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0, max_depth: 0, thread_count: 0, request_delay_ms: 0, max_retries: 0, request_timeout_sec: 0 };

        // Setup mock HTML page
        Mock::given(method("GET"))
            .and(path("/with-links"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"
            <html>
                <body>
                    <a href="/link1">Link 1</a>
                    <a href="https://example.com/link2">Link 2</a>
                    <a href="../link3">Link 3</a>
                </body>
            </html>
        "#))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&format!("{}/with-links", &mock_server.uri()))?;
        let result = scrape_page(url.clone(), &client, &config).await?;

        let expected = vec![
            construct_url("link1", (&mock_server.uri()).parse().unwrap())?,
            construct_url("../link3", (&mock_server.uri()).parse().unwrap())?,
        ];

        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_404() -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = Url::parse("https://example.com/not-found")?;
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0, max_depth: 0, thread_count: 0, request_delay_ms: 0, max_retries: 0, request_timeout_sec: 0 };
        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let url = Url::parse(&format!("{}/not-found", &mock_server.uri()))?;
        let result = scrape_page(url, &client, &config).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_timeout() -> Result<(), Box<dyn std::error::Error>> {
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0, max_depth: 0, thread_count: 0, request_delay_ms: 0, max_retries: 0, request_timeout_sec: 0 };
        let client = reqwest::Client::new();
        let url = Url::parse("https://example.com/timeout")?;

        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/timeout"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(LINK_REQUEST_TIMEOUT_SEC + 1)))
            .mount(&mock_server)
            .await;

        let result = scrape_page(url, &client, &config).await;
        assert!(result.is_err());
        Ok(())
    }
}
// tests for `scrape page` end here