use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role};
use tera::Context;

use crate::repository::CrawlerReader;
use crate::repository::crawler::DieselCrawlerRepository;
use crate::routes::render_template;

#[get("/")]
pub async fn index(
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

    let repo = DieselCrawlerRepository::new(&pool);

    let crawlers = match repo.list(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("crawlers", &crawlers);

    render_template("main/index.html", &context)
}

#[get("/na")]
pub async fn not_assigned(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);

    render_template("main/not_assigned.html", &context)
}
