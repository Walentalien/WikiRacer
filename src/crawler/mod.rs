pub mod state;
pub mod config;
pub mod scrape;
pub mod runner;

#[cfg(test)]
mod tests;

pub use state::{CrawlerState, CrawlerStateRef, build_graph_from_state};
pub use config::{CrawlerConfig, CrawlerConfigRef, LINK_REQUEST_TIMEOUT_SEC};
pub use scrape::{construct_url, scrape_page};
pub use runner::crawl;