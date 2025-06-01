mod crawler;
mod pathfinder;
mod integration_test;
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
        //.module_filter(|module| module.contains(""))
        .module_filter(|module| !module.starts_with("html5ever"))
        .compress(false)
        //.format(|record, tee| format!("[{}] [{}] {}\n", chrono::Local::now(), record.level(), record.args()))
        .start();
    info!("logging Initialized");

    let args: Vec<String> = std::env::args().collect();
    for arg in &args[..] {
        info!("arg: {}", arg);
    }


    if args.len() >= 3 {
        let start_url = url::Url::parse(&args[1])?;
        let target_url = url::Url::parse(&args[2])?;

        let config = std::sync::Arc::new(
            crawler::CrawlerConfig::new(start_url.clone())
                .with_max_urls(2000)
                .with_max_depth(3)
                .with_thread_count(8)
                .with_request_delay(100)
        );

        let state = std::sync::Arc::new(crawler::CrawlerState::new(start_url.clone()));

        info!("Starting crawler with {} threads, max {} URLs, max depth {}",
              config.thread_count, config.max_urls, config.max_depth);
        info!("Finding path from {} to {}", start_url, target_url);

        match crawler::crawl(state.clone(), config).await {
            Ok(_) => {
                let final_count = state.links_crawled_count.load(std::sync::atomic::Ordering::Relaxed);
                info!("Crawling completed successfully. Total links found: {}", final_count);

                // Build graph from crawled data
                let graph = crawler::build_graph_from_state(&state);

                // Find shortest path
                match crate::pathfinder::find_shortest_path(&start_url, &target_url, &graph) {
                    Some(path) => {
                        info!("Path found!");
                        crate::pathfinder::print_path(&path);
                        info!("Number of links between pages: {}", path.len() - 1);
                    }
                    None => {
                        info!("No path found between {} and {}", start_url, target_url);
                    }
                }
            }
            Err(e) => {
                error!("Crawling failed: {}", e);
            }
        }
    } else {
        info!("Usage: {} <start_url> <target_url>", args[0]);
        info!("Example: {} https://en.wikipedia.org/wiki/Rust_(programming_language) https://en.wikipedia.org/wiki/C_(programming_language)", args[0]);
    }

    Ok(())
}

