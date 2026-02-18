use serde::{Deserialize, Serialize};

/// Messages received over ZMQ to control crawlers or run benchmarks.
///
/// - `Crawler` requests execution of a crawler described by [`CrawlerSelector`].
/// - `Benchmark` triggers a benchmark run with the provided benchmark_id.
#[derive(Serialize, Deserialize, Debug)]
pub enum ZMQCrawlerMessage {
    /// Run the specified crawler.
    Crawler(CrawlerSelector),
    /// Execute benchmarks with the provided benchmark_id.
    Benchmark(i32),
}

/// Selects a crawler and optionally a list of product URLs to crawl.
///
/// - `Selector` chooses a crawler by name.
/// - `SelectorProducts` specifies a crawler and products to fetch.
#[derive(Serialize, Deserialize, Debug)]
pub enum CrawlerSelector {
    /// Run the named crawler.
    Selector(String),
    /// Run the named crawler with the provided product URLs.
    SelectorProducts((String, Vec<String>)),
}
