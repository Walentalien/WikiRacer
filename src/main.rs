mod crawler;
mod pathfinder;
mod integration_test;
mod config;

use log2::*;
use anyhow::Result;
use url::Url;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Init logger
    let _log2 = //open("log/log.txt")
        //.tee(true)
    stdout()
        .module(true)
        .module_with_line(true)
        .module_filter(|module| module.starts_with("WikiRacer"))
        .compress(false)
        .level("trace")
        .start();
    info!("Logging initialized");

    // Load CLI config
    let cfg = config::Config::new();
    if cfg.verbose {
        info!("Loaded config: {:?}", cfg);
    }
    cfg.validate()?;

    let start_url = Url::parse(&cfg.start_url)?;
    let target_url = Url::parse(&cfg.target_url)?;

    let crawler_config = Arc::new(
        crawler::CrawlerConfig::new(start_url.clone())
            .with_max_urls(cfg.max_urls)
            .with_max_depth(cfg.max_depth)
            .with_thread_count(cfg.thread_count)
            .with_request_delay(cfg.request_delay)
            .with_target_url(target_url.clone()),
    );

    let state = Arc::new(crawler::CrawlerState::new(start_url.clone()));

    info!(
        "Starting crawler with {} threads, max {} URLs, max depth {}",
        crawler_config.thread_count, crawler_config.max_urls, crawler_config.max_depth
    );
    info!("Finding path from {} to {}", start_url, target_url);

    match crawler::crawl(state.clone(), crawler_config).await {
        Ok(_) => {
            let final_count = state.links_crawled_count.load(std::sync::atomic::Ordering::Relaxed);
            info!("Crawling completed. Total links found: {}", final_count);

            let graph = crawler::build_graph_from_state(&state);

            match pathfinder::find_shortest_path_bfs(&start_url, &target_url, &graph) {
                Some(path) => {
                    info!("Path found!");
                    pathfinder::print_path(&path);
                    info!("Number of links between pages: {}", path.len() - 1);
                }
                None => {
                    info!("No path found between {} and {}", start_url, target_url);
                }
            }

            if let Some(path) = cfg.output_file {
                std::fs::write(&path, format!("{:?}", graph))?;
                info!("Graph written to {:?}", path);
            }
        }
        Err(e) => {
            error!("Crawling failed: {}", e);
        }
    }

    Ok(())
}

