use anyhow::{anyhow, Result};
use log2::{debug, info};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::time::Duration;
use url::Url;

use super::config::CrawlerConfig;

/// If `path` is a full URL, returns it as-is. Otherwise constructs a full URL by
/// merging with `root_url`. Trailing slashes are removed and fragments stripped.
pub fn construct_url(path: &str, root_url: Url) -> Result<Url, url::ParseError> {
    let mut url = if let Ok(parsed_url) = Url::parse(path) {
        if parsed_url.host().is_some() {
            parsed_url
        } else {
            root_url.join(path)?
        }
    } else {
        root_url.join(path)?
    };

    let trimmed_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&trimmed_path);
    url.set_fragment(None);

    Ok(url)
}

/// Scrape all links from the given page.
pub async fn scrape_page(url: Url, client: &Client, config: &CrawlerConfig) -> Result<HashSet<Url>> {
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
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(parsed_url) = construct_url(href, url.clone()) {
                let should_include = if config.scraping_foreign_hosts {
                    true
                } else {
                    match (base_host, parsed_url.host_str()) {
                        (Some(base), Some(target)) => base == target,
                        _ => false,
                    }
                };

                if should_include {
                    found_urls.insert(parsed_url);
                } else {
                    debug!("Skipped foreign host link: {}", parsed_url);
                }
            }
        }
    }

    info!("Found {} urls on page {}", found_urls.len(), url);

    Ok(found_urls)
}