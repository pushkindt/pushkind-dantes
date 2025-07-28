use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::crawler::Crawler;

pub mod crawler;

pub trait CrawlerReader {
    fn list(&mut self) -> RepositoryResult<Vec<Crawler>>;
    fn get_by_id(&mut self, id: i32) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {
    fn set_processing(&mut self, id: i32, status: bool) -> RepositoryResult<usize>;
}
