use std::collections::{VecDeque, HashMap, HashSet};
use std::fs;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool} ;
use std::time::Duration;
use log2::{debug, info, trace};
use url::Url;
use reqwest::Client;
use anyhow::{Result, anyhow};
use scraper::{Html, Selector};
use serde::Serialize;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/*
Macro for debugging purposes that writes links to the file;
I want to see if i'm getting any duplicates
 */
#[macro_export]
macro_rules! write_url {
    // Case: HashSet or iterable
    ($path:expr, [$($url:expr),+ $(,)?]) => {{
        #[cfg(debug_assertions)] // doesnt run in release
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::time::Duration;

            let elapsed = $crate::START_TIME.elapsed();
            let minutes = elapsed.as_secs() / 60;
            let seconds = elapsed.as_secs() % 60;

            let result = OpenOptions::new()
                .create(true)
                .append(true)
                .open($path)
                .and_then(|mut file| {
                    $(
                        writeln!(file, "[{:02}:{:02}] {}", minutes, seconds, $url)?;
                    )+
                    Ok(())
                });

            if let Err(e) = result {
                eprintln!("Failed to write to {}: {}", $path, e);
            }
        }
    }};

    // Case: single Url
    ($path:expr, $url:expr) => {{
        #[cfg(debug_assertions)]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::time::Duration;

            let elapsed = $crate::START_TIME.elapsed();
            let minutes = elapsed.as_secs() / 60;
            let seconds = elapsed.as_secs() % 60;

            let result = OpenOptions::new()
                .create(true)
                .append(true)
                .open($path)
                .and_then(|mut file| {
                    writeln!(file, "[{:02}:{:02}] {}", minutes, seconds, $url)
                });

            if let Err(e) = result {
                eprintln!("Failed to write to {}: {}", $path, e);
            }
        }
    }};
}



pub const LINK_REQUEST_TIMEOUT_SEC: u64 = 2;

/// Configuration of current state of the crawler
pub struct CrawlerState {
    /// Number of crawled links
    pub links_crawled_count: AtomicUsize,
    /// Links to crawl in queue
    pub link_to_crawl_queue: RwLock<VecDeque<(Url, usize)>>, // (url, depth)
    /// Set of visited URL to prevent cycles in graph
    pub visited_urls: RwLock<std::collections::HashSet<Url>>,
    /// Graph Structure to visualize the result later
    /// TODO: Create visualization
    pub url_relationships: RwLock<HashMap<Url, HashSet<Url>>>, // parent -> children
    /// Indicator that target link has been found
    pub target_found: AtomicBool,
}

impl CrawlerState {
    pub fn new(starting_url: Url) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((starting_url, 0)); // Start at depth 0

        Self {
            links_crawled_count: AtomicUsize::new(0),
            link_to_crawl_queue: RwLock::new(queue),
            visited_urls: RwLock::new(std::collections::HashSet::new()),
            url_relationships: RwLock::new(HashMap::new()),
            target_found: AtomicBool::new(false),
        }
    }
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
    /// Ultimatively it defines number of iterations in bfs
    pub max_depth: usize,
    /// number of worker threads for concurrent crawling
    pub thread_count: usize,
    /// delay between requests in milliseconds (to be respectful to servers)
    /// Since i'm scraping Wikipedia w/o using appropriate APIs, i don't want to get banned
    pub request_delay_ms: u64,
    /// maximum number of retries for failed requests
    pub max_retries: usize,
    /// timeout for each request in seconds
    pub request_timeout_sec: u64,
    /// the url we are looking for
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

    // Setters for config for testing purposes

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
    pub fn  with_target_url(mut self, url: Url) -> Self{
    self.target_url = Some(url);
    self
    }
}

type CrawlerConfigRef = Arc<CrawlerConfig>;


/// If `path` is full_url -> returns it
/// if relative constructs full url  by merging with `root_url`
/// If there is trailing slash it removes it
pub(crate) fn construct_url(path: &str, root_url: Url) -> Result<Url, url::ParseError> {
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

    //trace!("Constructed link URL: {}", url);

    write_url!("log.txt", &url);
    Ok(url)

}

/// Reads one link from `link_to_crawl_queue` and scrapes all links from there to the end of that queue
/// Returns: All found links on the page
pub async fn scrape_page(url: Url, client: &Client, config: &CrawlerConfig) -> Result<HashSet<Url>> {
    trace!("Scraping page: {}", url);

    let response = client
        .get(url.clone())
        .timeout(Duration::from_secs(config.request_timeout_sec))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch page: {}", response.status()));
    }

    let html = response.text().await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a")
        .map_err(|e| anyhow!("Failed to parse <a> selector: {}", e))?;

    let mut found_urls = HashSet::new();
    let base_host = url.host_str();
    // for every parsed element in html
    for element in document.select(&selector) {
        //if it's has href attribute (is a link)
        if let Some(href) = element.value().attr("href") {
            // construct a url
            if let Ok(parsed_url) = construct_url(href, url.clone()) {
                // Foreign host check
                let should_include = if config.scraping_foreign_hosts {
                    true
                } else {
                    match (base_host, parsed_url.host_str()) {
                        (Some(base), Some(target)) => base == target,
                        _ => false, // skip if either host is missing
                    }
                };

                if should_include {
                    write_url!("scrape_page_log.txt", &parsed_url);
                    found_urls.insert(parsed_url);
                } else {
                    debug!("Skipped foreign host link: {}", parsed_url);
                }
            }
        }
    }

    // Maybe will cahnge foreign host scraping logic to be this:
    // if !config.scraping_foreign_hosts {
    //     found_urls.retain(|u| u.host_str() == base_host);
    // }

    info!("Found {} urls on page {}", found_urls.len(), url);

    Ok(found_urls)
}

pub async fn crawl(crawler_state_ref: CrawlerStateRef, crawler_cfg_ref: CrawlerConfigRef) -> Result<()> {
    let active_workers = Arc::new(AtomicUsize::new(0));
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for worker_id in 0..crawler_cfg_ref.thread_count {
        let state = Arc::clone(&crawler_state_ref);
        let config = Arc::clone(&crawler_cfg_ref);
        let active_workers = Arc::clone(&active_workers);

        let handle = tokio::spawn(async move {
            let client = Client::new();
            info!("Worker {} started", worker_id);

            loop {
                if state.target_found.load(Ordering::SeqCst){
                    info!("Worker {}: Target has been found. Exiting...", worker_id);
                    break;
                }
                let next_item = {
                    let mut queue = state.link_to_crawl_queue.write().unwrap();
                    queue.pop_front()
                };

                if let Some((url, depth)) = next_item {
                    active_workers.fetch_add(1, Ordering::SeqCst);

                    if depth >= config.max_depth {
                        debug!("Worker {}: Max depth {} reached for {}", worker_id, config.max_depth, url);
                        active_workers.fetch_sub(1, Ordering::SeqCst);
                        continue;
                    }

                    {
                        let mut visited = state.visited_urls.write().unwrap();
                        if visited.contains(&url) {
                            active_workers.fetch_sub(1, Ordering::SeqCst);
                            continue;
                        }
                        visited.insert(url.clone());
                    }

                    if state.links_crawled_count.load(Ordering::Relaxed) >= config.max_urls {
                        info!("Worker {}: Max URLs reached", worker_id);
                        active_workers.fetch_sub(1, Ordering::SeqCst);
                        break;
                    }

                    info!("Worker {}: Crawling {} at depth {}", worker_id, url, depth);

                    match scrape_page(url.clone(), &client, &config).await {
                        Ok(found_urls) => {
                            {
                                let mut relationships = state.url_relationships.write().unwrap();
                                relationships.insert(url.clone(), found_urls.clone());
                            }

                            {

                                if let Some(target) = &config.target_url {
                                    if found_urls.iter().any(|u| u == target) {
                                        state.target_found.store(true, Ordering::SeqCst);
                                    }
                                }
                                let mut queue = state.link_to_crawl_queue.write().unwrap();
                                for found_url in found_urls {
                                    queue.push_back((found_url, depth + 1));
                                }
                            }

                            state.links_crawled_count.fetch_add(1, Ordering::Relaxed);

                            if config.request_delay_ms > 0 {
                                sleep(Duration::from_millis(config.request_delay_ms)).await;
                            }
                        }
                        Err(e) => {
                            debug!("Worker {}: Failed to scrape {}: {}", worker_id, url, e);
                        }
                    }

                    active_workers.fetch_sub(1, Ordering::SeqCst);
                } else {
                    sleep(Duration::from_millis(200)).await;

                    let queue_empty = {
                        let queue = state.link_to_crawl_queue.read().unwrap();
                        queue.is_empty()
                    };

                    let idle = active_workers.load(Ordering::SeqCst) == 0;

                    if queue_empty && idle {
                        info!("Worker {}: Queue empty and all workers idle. Shutting down.", worker_id);
                        break;
                    }
                }
            }

            info!("Worker {} finished", worker_id);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

/// Build a graph from the crawled state for pathfinding
pub fn build_graph_from_state(state: &CrawlerStateRef) -> HashMap<Url, HashSet<Url>> {
    let relationships = state.url_relationships.read().unwrap();
    relationships.clone()
}



#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use super::*;
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
    ///Checks if sections are removed
    #[test]
    fn test_hash_fragment() -> Result<(), Box<dyn std::error::Error>> {
        let root_url = Url::parse("https://groq.xyz")?;
        let relative_path = "/some/relative/path#section/";
        let result_url = construct_url(relative_path, root_url)?;
        let expected_url = Url::parse("https://groq.xyz/some/relative/path")?;
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

    #[test]
    fn test_url_normalization() {
        use crate::crawler::construct_url;
        use url::Url;

        let base = Url::parse("https://en.wikipedia.org/wiki/Matter").unwrap();

        // Test fragment removal
        let url1 = construct_url("/wiki/Grand_Unified_Theory#History", base.clone()).unwrap();
        let url2 = construct_url("/wiki/Grand_Unified_Theory", base.clone()).unwrap();

        assert_eq!(url1, url2, "URLs with and without fragments should be equal");

        // Test trailing slash removal
        let url3 = construct_url("/wiki/Grand_Unified_Theory/", base.clone()).unwrap();
        assert_eq!(url2, url3, "URLs with and without trailing slash should be equal");
    }
    // tests for construct_url end here

    // tests for `scrape page` start here

    #[tokio::test]
    async fn test_scrape_empty_page() -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        //let url = Url::parse("https://example.com/empty")?;
        let config: CrawlerConfig = CrawlerConfig {
            starting_url: Url::parse("https://localhost")?,
            scraping_foreign_hosts: false,
            max_urls: 0,
            max_depth: 0,
            thread_count: 0,
            request_delay_ms: 0,
            max_retries: 0,
            request_timeout_sec: 2,
            target_url: None,
        };

        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/empty"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&mock_server)
            .await;
        let url = Url::parse(&format!("{}/empty", &mock_server.uri()))?;
        let result = scrape_page(url, &client, &config).await;
        assert_eq!(result?, HashSet::<Url>::new());
        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_with_links() -> Result<(), Box<dyn std::error::Error>> {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use url::Url;

        let client = reqwest::Client::new();
        let mock_server = MockServer::start().await;
        let config: CrawlerConfig = CrawlerConfig {
            starting_url: Url::parse("https://localhost")?,
            scraping_foreign_hosts: false,
            max_urls: 0,
            max_depth: 0,
            thread_count: 0,
            request_delay_ms: 0,
            max_retries: 0,
            request_timeout_sec: 2,
            target_url: None,
        };
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
        //collected form a vec
        let expected: HashSet<Url> = vec![
            construct_url("link1", (&mock_server.uri()).parse().unwrap())?,
            construct_url("../link3", (&mock_server.uri()).parse().unwrap())?,
        ].into_iter().collect();
        //alternative to collecting:
        // let expected: HashSet<Url> = HashSet::from_iter([
        //     construct_url("link1", (&mock_server.uri()).parse().unwrap())?,
        //     construct_url("../link3", (&mock_server.uri()).parse().unwrap())?,
        // ]);


        assert_eq!(result, expected);

        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_404() -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = Url::parse("https://example.com/not-found")?;
        let config: CrawlerConfig = CrawlerConfig {
            starting_url: Url::parse("https://localhost")?,
            scraping_foreign_hosts: false,
            max_urls: 0,
            max_depth: 0,
            thread_count: 0,
            request_delay_ms: 0,
            max_retries: 0,
            request_timeout_sec: 2,
            target_url: None,
        };
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
        //let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0, max_depth: 0, thread_count: 0, request_delay_ms: 0, max_retries: 0, request_timeout_sec: 0 };
        let config: CrawlerConfig = CrawlerConfig {
            starting_url: Url::parse("https://localhost")?,
            scraping_foreign_hosts: false,
            max_urls: 0,
            max_depth: 0,
            thread_count: 0,
            request_delay_ms: 0,
            max_retries: 0,
            request_timeout_sec: 2,
            target_url: None,
        };
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
    // end of test section for `scrape_page`


    // test suite  for   `crawl` start here
    fn test_config(start_url: Url) -> CrawlerConfigRef {
        Arc::new(CrawlerConfig {
            starting_url: start_url,
            scraping_foreign_hosts: false,
            max_urls: 10,
            max_depth: 2,
            thread_count: 2,
            request_delay_ms: 0,
            max_retries: 0,
            request_timeout_sec: 5,
            target_url: None,
        })
    }

    fn test_state(start_url: Url) -> CrawlerStateRef {
        Arc::new(CrawlerState::new(start_url))
    }

    /// What i learned from this test:
    /// `crawl` adds links that don't have children to the queue too
    #[tokio::test]
    async fn test_crawl_basic() {
        let server = MockServer::start().await;
        let page = format!("{}/start", server.uri());

        Mock::given(path("/start"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"
                <a href="/a">A</a>
                <a href="/b">B</a>
            "#))
            .mount(&server).await;

        Mock::given(path("/a"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server).await;

        Mock::given(path("/b"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server).await;

        let url = Url::parse(&page).unwrap();
        let config = test_config(url.clone());
        let state = test_state(url.clone());

        crawl(state.clone(), config.clone()).await.unwrap();

        let graph = state.url_relationships.read().unwrap();
        assert_eq!(graph.len(), 3); // Only root has children
        // https://localhost/start :  https://localhost/a https://localhost/b <- what is expected to be in graph
        // Root has two children
        assert_eq!(graph.get(&url).unwrap().len(), 2);
    }
/// What i learned form this test:
/// At `max_depth` of 1 only starting page is crawled
/// So at `max_depth` of 0 crawl will exit immediately
#[tokio::test]
async fn test_max_depth_respected() {
    let server = MockServer::start().await;

    Mock::given(path("/root"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"<a href="/child1">Next</a>"#))
        .mount(&server)
        .await;

    Mock::given(path("/child1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"<a href="/child2">Deep</a>"#))
        .mount(&server)
        .await;

    let url = Url::parse(&format!("{}/root", server.uri())).unwrap();

    let config = Arc::new(
        CrawlerConfig::new(url.clone())
            .with_max_depth(1)
            .with_max_urls(10)
            .with_thread_count(1)
            .with_request_delay(0),
    );

    let state = test_state(url.clone());
    crawl(state.clone(), config).await.unwrap();

    let visited = state.visited_urls.read().unwrap();
    assert!(visited.contains(&url));
    assert!(!visited.iter().any(|u| u.path().contains("child1"))); // Not crawled

    let graph = state.url_relationships.read().unwrap();
    let children = graph.get(&url).unwrap();
    assert!(children.iter().any(|u| u.path().contains("child1"))); // Discovered, but not crawled
}

    #[tokio::test]
    async fn test_max_urls_respected() {
        let server = MockServer::start().await;
        let url = Url::parse(&format!("{}/root", server.uri())).unwrap();

        let mut html = String::new();
        for i in 0..20 {
            html += &format!(r#"<a href="/link{}">Link</a>"#, i);
            Mock::given(path(&format!("/link{}", i)))
                .respond_with(ResponseTemplate::new(200).set_body_string(""))
                .mount(&server)
                .await;
        }

        Mock::given(path("/root"))
            .respond_with(ResponseTemplate::new(200).set_body_string(&html))
            .mount(&server)
            .await;

        let config = Arc::new(
            CrawlerConfig::new(url.clone())
                .with_max_urls(5)
                .with_max_depth(2)
                .with_thread_count(2)
                .with_request_delay(0),
        );

        let state = test_state(url.clone());
        crawl(state.clone(), config).await.unwrap();

        let count = state.links_crawled_count.load(Ordering::Relaxed);
        assert_eq!(count, 5);
    }


    /// Checks that foreign hosted link is not scraped
    /// (Doesn't appear in the graph)
    #[tokio::test]
    async fn test_ignores_foreign_hosts() {
        let server = MockServer::start().await;
        let root = format!("{}/page", server.uri());
        let foreign = "https://google.com";

        Mock::given(path("/page"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(r#"
                <a href="{}">Foreign</a>
                <a href="/local">Local</a>
            "#, foreign)))
            .mount(&server).await;

        Mock::given(path("/local"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server).await;

        let url = Url::parse(&root).unwrap();
        let config = test_config(url.clone());
        let state = test_state(url.clone());

        crawl(state.clone(), config).await.unwrap();

        let graph = state.url_relationships.read().unwrap();
        assert!(graph.get(&url).unwrap().iter().all(|u| u.domain() != Some("google.com")));
    }

    #[tokio::test]
    async fn test_handles_http_error() {
        let server = MockServer::start().await;
        let url = Url::parse(&format!("{}/404", server.uri())).unwrap();

        Mock::given(path("/404"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server).await;

        let config = test_config(url.clone());
        let state = test_state(url.clone());

        crawl(state.clone(), config).await.unwrap();

        let visited = state.visited_urls.read().unwrap();
        assert!(visited.contains(&url));

        let graph = state.url_relationships.read().unwrap();
        assert!(graph.get(&url).is_none()); // No children on error
    }
    // tests for `scrape page` end here
}

