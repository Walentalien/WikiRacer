use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
/// Log levels as defined in log2 crate
#[derive(Debug, Serialize, Deserialize, Clone, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
/// This struct is supposed to receive all program arguments while Crawlerconfig
/// Describes only The the Crawler
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
    #[arg(long, default_value = "10000")]
    pub max_urls: usize,
    /// Maximum depth to crawl
    #[arg(long, default_value = "5")]
    pub max_depth: usize,
    /// Number of threads to use for crawling
    #[arg(long, default_value = "8")]
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
    /// Logging level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", value_enum)]
    pub log_level: LogLevel,
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

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        write!(f, "{}", s)
    }
}