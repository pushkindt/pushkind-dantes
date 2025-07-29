use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role};
use tera::Context;

use crate::repository::product::DieselProductRepository;
use crate::repository::ProductReader;
use crate::routes::render_template;

#[get("/crawler/{crawler_id}/products")]
pub async fn show_products(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

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

    let products = match repo.list(crawler_id.into_inner()) {
        Ok(products) => products,
        Err(e) => {
            log::error!("Failed to list products: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("products", &products);

    render_template("products/index.html", &context)
}
