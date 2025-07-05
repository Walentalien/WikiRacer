use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use tokio::sync::RwLock;
use url::Url;

/// Current state of the crawler
pub struct CrawlerState {
    /// Number of crawled links
    pub links_crawled_count: AtomicUsize,
    /// Links to crawl in queue
    pub link_to_crawl_queue: RwLock<VecDeque<(Url, usize)>>,
    /// Set of visited URLs to prevent cycles
    pub visited_urls: RwLock<HashSet<Url>>,
    /// Graph structure to visualize the result later
    pub url_relationships: RwLock<HashMap<Url, HashSet<Url>>>,
    /// Indicator that target link has been found
    pub target_found: AtomicBool,
}

impl CrawlerState {
    pub fn new(starting_url: Url) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((starting_url, 0));

        Self {
            links_crawled_count: AtomicUsize::new(0),
            link_to_crawl_queue: RwLock::new(queue),
            visited_urls: RwLock::new(HashSet::new()),
            url_relationships: RwLock::new(HashMap::new()),
            target_found: AtomicBool::new(false),
        }
    }
}

pub type CrawlerStateRef = Arc<CrawlerState>;

/// Build a graph from the crawled state for pathfinding
pub fn build_graph_from_state(state: &CrawlerStateRef) -> HashMap<Url, HashSet<Url>> {
    let relationships = state.url_relationships.blocking_read();
    relationships.clone()
}