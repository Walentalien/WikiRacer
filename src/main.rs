// src/main.rs
mod crawler;

use std::collections::{HashMap, HashSet};
use crawler::{build_graph_from_state, CrawlerConfig, CrawlerState};
use log2::*;
use anyhow::Result;
use url::Url;
use std::sync::Arc;
use std::time::Instant;
use WikiRacer::pathfinder;
use WikiRacer::config::Config;

/// Indicates start time of a project, lazily initialized
pub static START_TIME: once_cell::sync::Lazy<Instant> =
    once_cell::sync::Lazy::new(Instant::now);

#[tokio::main]
async fn main() -> Result<()> {
    // initialize timer
    let _ = *START_TIME;

    // load & validate config
    let cfg = Config::new();
    cfg.validate()?;

    // set up logging
    let _log2 = stdout()
        .module(true)
        .module_with_line(true)
        .module_filter(|m| m.starts_with("WikiRacer"))
        .compress(false)
        .level(cfg.log_level.to_string())
        .start();

    // parse URLs
    let start_url = Url::parse(&cfg.start_url)?;
    let target_url = Url::parse(&cfg.target_url)?;

    // build crawler config & state
    let crawler_cfg = Arc::new(
        CrawlerConfig::new(start_url.clone())
            .with_max_urls(cfg.max_urls)
            .with_max_depth(cfg.max_depth)
            .with_thread_count(cfg.thread_count)
            .with_request_delay(cfg.request_delay)
            .with_target_url(target_url.clone()),
    );
    let state = Arc::new(CrawlerState::new(start_url.clone()));

    // run crawl
    match crawler::crawl(state.clone(), crawler_cfg).await {
        Ok(_) => {
            let count = state.links_crawled_count.load(std::sync::atomic::Ordering::Relaxed);
            debug!("Crawling completed. Total links found: {}", count);

            // build graph synchronously
            let graph: HashMap<Url, HashSet<Url>> = build_graph_from_state(&state).await;

            // find shortest path
            if let Some(path) =
                pathfinder::find_shortest_path_bfs(&start_url, &target_url, &graph)
            {
                info!("Path found!");
                pathfinder::print_path(&path);
                info!("Number of links between pages: {}", path.len() - 1);
            } else {
                info!("No path found between {} and {}", start_url, target_url);
            }

            // optionally write full graph dump
            if let Some(path) = cfg.output_file {
                std::fs::write(&path, format!("{:#?}", graph))?;
                info!("Graph written to {:?}", path);
            }
        }
        Err(e) => error!("Crawling failed: {}", e),
    }

    Ok(())
}
