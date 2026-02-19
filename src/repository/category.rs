use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::category::{Category, NewCategory};
use crate::domain::types::{CategoryAssignmentSource, CategoryId, CategoryName, HubId};
use crate::models::category::{Category as DbCategory, NewCategory as DbNewCategory};
use crate::repository::{CategoryListQuery, CategoryReader, CategoryWriter, DieselRepository};

impl CategoryReader for DieselRepository {
    fn list_categories(
        &self,
        query: CategoryListQuery,
    ) -> RepositoryResult<(usize, Vec<Category>)> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let query_builder = || {
            categories::table
                .filter(categories::hub_id.eq(query.hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>()
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();
        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let items = items
            .order(categories::name.asc())
            .load::<DbCategory>(&mut conn)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Category>, _>>()?;

        Ok((total, items))
    }

    fn get_category_by_id(
        &self,
        id: CategoryId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Category>> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let category = categories::table
            .filter(categories::id.eq(id.get()))
            .filter(categories::hub_id.eq(hub_id.get()))
            .first::<DbCategory>(&mut conn)
            .optional()?;

        let category = category.map(TryInto::try_into).transpose()?;
        Ok(category)
    }
}

impl CategoryWriter for DieselRepository {
    fn create_category(&self, category: &NewCategory) -> RepositoryResult<usize> {
        use crate::schema::categories;

        let mut conn = self.conn()?;
        let db_category: DbNewCategory = category.clone().into();

        let affected = diesel::insert_into(categories::table)
            .values(db_category)
            .execute(&mut conn)?;

        Ok(affected)
    }

    fn update_category(
        &self,
        id: CategoryId,
        hub_id: HubId,
        name: &CategoryName,
        embedding: Option<&[u8]>,
    ) -> RepositoryResult<usize> {
        use crate::schema::categories;

        let mut conn = self.conn()?;

        let affected = diesel::update(
            categories::table
                .filter(categories::id.eq(id.get()))
                .filter(categories::hub_id.eq(hub_id.get())),
        )
        .set((
            categories::name.eq(name.as_str()),
            categories::embedding.eq(embedding),
            categories::updated_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)?;

        Ok(affected)
    }

    fn delete_category(&self, id: CategoryId, hub_id: HubId) -> RepositoryResult<usize> {
        use crate::schema::{categories, crawlers, products};

        let mut conn = self.conn()?;

        let affected = conn.transaction(|conn| {
            diesel::update(
                products::table
                    .filter(products::category_id.eq(Some(id.get())))
                    .filter(
                        products::crawler_id.eq_any(
                            crawlers::table
                                .filter(crawlers::hub_id.eq(hub_id.get()))
                                .select(crawlers::id),
                        ),
                    ),
            )
            .set(
                products::category_assignment_source
                    .eq(CategoryAssignmentSource::Automatic.as_str()),
            )
            .execute(conn)?;

            diesel::delete(
                categories::table
                    .filter(categories::id.eq(id.get()))
                    .filter(categories::hub_id.eq(hub_id.get())),
            )
            .execute(conn)
        })?;

        Ok(affected)
    }
}
