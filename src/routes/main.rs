use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use pushkind_common::zmq::send_zmq_message;
use tera::Context;

use crate::models::config::ServerConfig;
use crate::repository::crawler::DieselCrawlerRepository;
use crate::repository::{CrawlerReader, CrawlerWriter};
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

#[post("/process/{crawler_id}")]
pub async fn process_crawler(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

    let crawler_id = crawler_id.into_inner();

    let repo = DieselCrawlerRepository::new(&pool);

    let crawler = match repo.get_by_id(crawler_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => {
            FlashMessage::error("Такого парсера не существует").send();
            return redirect("/");
        }
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match send_zmq_message(crawler.selector.as_bytes(), &server_config.zmq_address) {
        Ok(_) => {
            FlashMessage::success("Обработка запущена").send();
            match repo.set_processing(crawler_id, true) {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Failed to set crawler as processing: {e}");
                    FlashMessage::error("Не удалось установить флаг обработки.").send();
                }
            }
        }
        Err(e) => {
            log::error!("Failed to send ZMQ message: {e}");
            FlashMessage::error("Не удалось начать обработку.").send();
        }
    }

    redirect("/")
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
