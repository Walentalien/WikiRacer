use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Starting Wikipedia URL
    #[arg(short, long)]
    pub start_url: String,

    /// Target Wikipedia URL
    #[arg(long)]
    pub target_url: String,

    /// Maximum number of URLs to crawl
    #[arg( long, default_value = "1000")]
    pub max_urls: usize,

    /// Maximum depth to crawl
    #[arg( long, default_value = "3")]
    pub max_depth: usize,

    /// Number of threads to use for crawling
    #[arg( long, default_value = "4")]
    pub thread_count: usize,

    /// Delay between requests in milliseconds
    #[arg(short, long, default_value = "100")]
    pub request_delay: u64,

    /// Output file for the graph visualization
    #[arg(short, long)]
    pub output_file: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

impl Config {
    pub fn new() -> Self {
        Self::parse()
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_urls <= 0 {
            anyhow::bail!("max_urls must be greater than 0");
        }
        if self.max_depth <= 0 {
            anyhow::bail!("max_depth must be greater than 0");
        }
        if self.thread_count <= 0 {
            anyhow::bail!("thread_count must be greater than 0");
        }
        Ok(())
    }
}