use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::types::HubId;
use crate::repository::{DieselRepository, ProcessingStateReader};

impl ProcessingStateReader for DieselRepository {
    fn has_active_processing(&self, hub_id: HubId) -> RepositoryResult<bool> {
        use crate::schema::{benchmarks, crawlers};

        let mut conn = self.conn()?;

        let active_crawlers = crawlers::table
            .filter(crawlers::hub_id.eq(hub_id.get()))
            .filter(crawlers::processing.eq(true))
            .count()
            .get_result::<i64>(&mut conn)?
            > 0;

        if active_crawlers {
            return Ok(true);
        }

        let active_benchmarks = benchmarks::table
            .filter(benchmarks::hub_id.eq(hub_id.get()))
            .filter(benchmarks::processing.eq(true))
            .count()
            .get_result::<i64>(&mut conn)?
            > 0;

        Ok(active_benchmarks)
    }
}
