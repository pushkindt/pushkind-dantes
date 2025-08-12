use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::ensure_role;
use pushkind_common::routes::{base_context, render_template};
use tera::Tera;

use crate::repository::CrawlerReader;
use crate::repository::crawler::DieselCrawlerRepository;

#[get("/")]
pub async fn index(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &server_config.auth_service_url,
    );

    let repo = DieselCrawlerRepository::new(&pool);

    let crawlers = match repo.list_crawlers(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("crawlers", &crawlers);

    render_template(&tera, "main/index.html", &context)
}
