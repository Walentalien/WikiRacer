use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, RwLock};
use log2::{debug, info, trace};
use url::Url;
use reqwest::Client;
use anyhow::{Result, anyhow};
use scraper::{Html, Selector};
use serde::Serialize;

const LINK_REQUEST_TIMEOUT_SEC: u64 = 2;

/// Configuration of current state of the crawler
pub struct CrawlerState {
    pub links_crawled_count: usize,
    pub link_to_crawl_queue: RwLock<VecDeque<Url>>,
    pub links_graph: RwLock<Vec<Link>>
}
#[derive(Serialize)]
pub struct Link {
    pub id: LinkID,
    pub url: String,
    pub children: Vec<Link>,
    pub parents: Vec<Link>
}

pub type LinkID = usize;
// Arc for simultaneous access by multiple threads
pub type CrawlerStateRef = Arc<CrawlerState>;

pub struct CrawlerConfig{

    /// starting point of scraping
    pub starting_url: Url,
    /// if should scrape foreign hosts
    pub scraping_foreign_hosts: bool,
    /// max urls to scrape
    pub max_urls: usize,
}
type CrawlerConfigRef = Arc<CrawlerConfig>;

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

    // Normalize: trim trailing slashes from path and fragment
    let trimmed_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&trimmed_path);

    if let Some(frag) = url.fragment().map(|s| s.to_string()) {
        url.set_fragment(Some(frag.trim_end_matches('/')));
    }
    trace!("Constructed link URL: {}", url);
    Ok(url)

}

/// Reads one link from `link_to_crawl_queue` and scrapes all links from there to the end of that queue
async fn scrape_page(url: Url, client: &Client, config: CrawlerConfig) -> Result<Vec<Url>> {
    trace!("Scraping page: {}", url);

    let response = client
        .get(url.clone())
        .timeout(std::time::Duration::from_secs(LINK_REQUEST_TIMEOUT_SEC))
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


                // Only collect URLs from the same host is `CrawlerConfig.scraping_foreign_hosts==true'
                if config.scraping_foreign_hosts == true {

                    found_urls.push(parsed_url);
                }
                else if  parsed_url.host() == url.host() {
                    found_urls.push(parsed_url);
                }
                else {
                    // inform about skipping a link
                    debug!("Skipped scraping {parsed_url} because its foreign host");
                }
            }
        }
    }

    trace!("Found {} urls on page {}", found_urls.len(), url);
    Ok(found_urls)
}


pub fn crawl(crawler_state_ref: CrawlerStateRef,
            crawler_cfg_ref: CrawlerConfigRef,){
    let client = Client::new();

    'crawler: loop{
        let number_of_links_scraped = crawler_state_ref.links_crawled_count;
        if number_of_links_scraped > crawler_cfg_ref.max_urls {
            info!("Max links capasity reached");
            break 'crawler;
        }

    }
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
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0 };


        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/empty"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&mock_server)
            .await;
        let url = Url::parse(&format!("{}/empty", &mock_server.uri()))?;
        let result = scrape_page(url, &client, config).await;
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
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0 };

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
        let result = scrape_page(url.clone(), &client, config).await?;

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
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0 };
        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let result = scrape_page(url, &client, config).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_page_timeout() -> Result<(), Box<dyn std::error::Error>> {
        let config:CrawlerConfig = CrawlerConfig { starting_url: Url::parse("https://localhost")?, scraping_foreign_hosts: false, max_urls: 0 };
        let client = reqwest::Client::new();
        let url = Url::parse("https://example.com/timeout")?;

        let mock_server = wiremock::MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/timeout"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(LINK_REQUEST_TIMEOUT_SEC + 1)))
            .mount(&mock_server)
            .await;

        let result = scrape_page(url, &client, config).await;
        assert!(result.is_err());
        Ok(())
    }
}
    // tests for `scrape page` end here



