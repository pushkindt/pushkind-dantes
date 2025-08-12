use actix_web::{HttpResponse, Responder, get, web};
use log::error;
use pushkind_common::db::DbPool;
use pushkind_common::domain::product::Product;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::ensure_role;
use serde::Deserialize;

use crate::repository::{CrawlerReader, DieselRepository, ProductListQuery, ProductReader};

#[derive(Deserialize, Debug)]
struct ApiV1ProductsQueryParams {
    crawler_id: i32,
    query: Option<String>,
    page: Option<usize>,
}

#[get("/v1/products")]
pub async fn api_v1_products(
    params: web::Query<ApiV1ProductsQueryParams>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if ensure_role(&user, "parser", None).is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let crawler_repo = DieselRepository::new(&pool);

    let crawler = match crawler_repo.get_crawler_by_id(params.crawler_id) {
        Ok(Some(crawler)) if crawler.hub_id == user.hub_id => crawler,
        Err(e) => {
            error!("Failed to get crawler: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => return HttpResponse::NotFound().finish(),
    };

    let product_repo = DieselRepository::new(&pool);
    let mut list_query = ProductListQuery::default().crawler(crawler.id);

    let page = params.page.unwrap_or(1);

    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let result = match &params.query {
        Some(query) if !query.is_empty() => {
            list_query = list_query.search(query);
            product_repo.search_products(list_query)
        }
        _ => product_repo.list_products(list_query),
    };

    match result {
        Ok((_total, products)) => HttpResponse::Ok().json(
            products
                .into_iter()
                .map(|mut p| {
                    p.embedding = None;
                    p
                })
                .collect::<Vec<Product>>(),
        ),
        Err(e) => {
            error!("Failed to list products: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
