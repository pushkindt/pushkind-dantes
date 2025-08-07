use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::FlashMessage;
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::models::zmq::dantes::CrawlerSelector;
use pushkind_common::models::zmq::dantes::ZMQMessage;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use pushkind_common::zmq::send_zmq_message;
use serde::Deserialize;
use tera::Context;

use crate::models::config::ServerConfig;
use crate::repository::CrawlerReader;
use crate::repository::ProductListQuery;
use crate::repository::ProductReader;
use crate::repository::crawler::DieselCrawlerRepository;
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

    let product_repo = DieselProductRepository::new(&pool);
    let crawler_repo = DieselCrawlerRepository::new(&pool);

    let crawler_id = crawler_id.into_inner();

    let products = match product_repo.list(
        ProductListQuery::default()
            .crawler(crawler_id)
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

    let crawler = match crawler_repo.get_by_id(crawler_id) {
        Ok(crawler) => crawler,
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("products", &products);
    context.insert("crawler", &crawler);

    render_template("products/index.html", &context)
}

#[post("/crawler/{crawler_id}/crawl")]
pub async fn crawl_crawler(
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

    let message = ZMQMessage::Crawler(CrawlerSelector::Selector(crawler.selector));
    match send_zmq_message(&message, &server_config.zmq_address) {
        Ok(_) => {
            FlashMessage::success("Обработка запущена").send();
        }
        Err(e) => {
            log::error!("Failed to send ZMQ message: {e}");
            FlashMessage::error("Не удалось начать обработку.").send();
        }
    }

    redirect(&format!("/crawler/{crawler_id}"))
}
