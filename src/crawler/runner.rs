use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Result;
use log2::*;
use reqwest::Client;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use super::config::{CrawlerConfigRef};
use super::state::CrawlerStateRef;
use super::scrape::scrape_page;

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
                if state.target_found.load(Ordering::SeqCst) {
                    info!("Worker {}: Target has been found. Exiting...", worker_id);
                    break;
                }
                let next_item = {
                    let mut queue = state.link_to_crawl_queue.write().await;
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
                        let mut visited = state.visited_urls.write().await;
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
                                let mut relationships = state.url_relationships.write().await;
                                relationships.insert(url.clone(), found_urls.clone());
                            }

                            {
                                if let Some(target) = &config.target_url {
                                    if found_urls.iter().any(|u| u == target) {
                                        state.target_found.store(true, Ordering::SeqCst);
                                    }
                                }
                                let mut queue = state.link_to_crawl_queue.write().await;
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
                        let queue = state.link_to_crawl_queue.read().await;
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