use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{alert_level_to_str, ensure_role};
use serde::Deserialize;
use tera::Context;

use crate::repository::ProductListQuery;
use crate::repository::ProductReader;
use crate::repository::product::DieselProductRepository;
use crate::routes::render_template;

#[derive(Deserialize)]
struct ProductsQueryParams {
    page: Option<usize>,
}

#[get("/crawler/{crawler_id}")]
pub async fn show_products(
    params: web::Query<ProductsQueryParams>,
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

    let page = params.page.unwrap_or(1);

    let mut context = Context::new();

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();

    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);

    let repo = DieselProductRepository::new(&pool);

    let products = match repo.list(
        ProductListQuery::default()
            .crawler(crawler_id.into_inner())
            .paginate(page, DEFAULT_ITEMS_PER_PAGE),
    ) {
        Ok((total, products)) => {
            Paginated::new(products, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
        }
        Err(e) => {
            log::error!("Failed to list products: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("products", &products);

    render_template("products/index.html", &context)
}
