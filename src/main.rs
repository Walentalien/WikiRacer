mod crawler;
mod link_graph;

use log2::*;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _log2 = log2::open("log/log.txt")
        //.size(100*1024*1024)
        //.rotate(20)
        .tee(true)
        .module(true)
        .module_with_line(true)
        .module_filter(|module| module.contains(""))
        .compress(false)
        //.format(|record, tee| format!("[{}] [{}] {}\n", chrono::Local::now(), record.level(), record.args()))
        .start();
    info!("logging Initialized");

    let args: Vec<String> = std::env::args().collect();
    for arg in &args[..] {
        info!("arg: {}", arg);
    }


    if args.len() > 1 {
        let start_url = url::Url::parse(&args[1])?;

        let config = std::sync::Arc::new(
            crawler::CrawlerConfig::new(start_url.clone())
                .with_max_urls(100)
                .with_max_depth(2)
                .with_thread_count(4)
                .with_request_delay(200)
        );

        let state = std::sync::Arc::new(crawler::CrawlerState::new(start_url));

        info!("Starting crawler with {} threads, max {} URLs, max depth {}",
              config.thread_count, config.max_urls, config.max_depth);

        match crawler::crawl(state.clone(), config).await {
            Ok(_) => {
                let final_count = state.links_crawled_count.load(std::sync::atomic::Ordering::Relaxed);                info!("Crawling completed successfully. Total links found: {}", final_count);
            }
            Err(e) => {
                error!("Crawling failed: {}", e);
            }
        }
    } else {
        info!("Usage: {} <starting_url>", args[0]);
        info!("Example: {} https://example.com", args[0]);
    }

    Ok(())
}

