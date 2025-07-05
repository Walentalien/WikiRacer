use std::collections::HashSet; use std::time::Duration;
use url::Url; use std::sync::Arc; use std::sync::atomic::Ordering;
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

    let graph = state.url_relationships.read().await;
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

    let visited = state.visited_urls.read().await;
    assert!(visited.contains(&url));
    assert!(!visited.iter().any(|u| u.path().contains("child1"))); // Not crawled

    let graph = state.url_relationships.read().await;
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

    let graph = state.url_relationships.read().await;
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

    let visited = state.visited_urls.read().await;
    assert!(visited.contains(&url));

    let graph = state.url_relationships.read().await;
    assert!(graph.get(&url).is_none()); // No children on error
}
// tests for `scrape page` end here