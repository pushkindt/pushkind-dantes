use serde::{Deserialize, Serialize};

use crate::domain::types::{BenchmarkId, CrawlerSelectorValue, HubId, ProductUrl};

/// Messages received over ZMQ to control crawlers or run benchmarks.
///
/// - `Crawler` requests execution of a crawler described by [`CrawlerSelector`].
/// - `Benchmark` triggers a benchmark run with the provided benchmark_id.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ZMQCrawlerMessage {
    /// Run the specified crawler.
    Crawler(CrawlerSelector),
    /// Execute benchmarks with the provided benchmark_id.
    Benchmark(BenchmarkId),
    /// Run product-to-category matching for a hub.
    ///
    /// Worker contract: automatic matching must not overwrite products with
    /// `category_assignment_source = manual`.
    ProductCategoryMatch(HubId),
}

/// Selects a crawler and optionally a list of product URLs to crawl.
///
/// - `Selector` chooses a crawler by name.
/// - `SelectorProducts` specifies a crawler and products to fetch.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum CrawlerSelector {
    /// Run the named crawler.
    Selector(CrawlerSelectorValue),
    /// Run the named crawler with the provided product URLs.
    SelectorProducts((CrawlerSelectorValue, Vec<ProductUrl>)),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_product_category_match_message() {
        let message = ZMQCrawlerMessage::ProductCategoryMatch(HubId::new(42).unwrap());
        let value = serde_json::to_value(&message).unwrap();

        assert_eq!(value, serde_json::json!({ "ProductCategoryMatch": 42 }));
    }

    #[test]
    fn deserializes_product_category_match_message() {
        let value = serde_json::json!({ "ProductCategoryMatch": 42 });
        let parsed: ZMQCrawlerMessage = serde_json::from_value(value).unwrap();

        assert_eq!(
            parsed,
            ZMQCrawlerMessage::ProductCategoryMatch(HubId::new(42).unwrap())
        );
    }
}
