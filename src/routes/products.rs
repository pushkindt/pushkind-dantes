use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::models::zmq::dantes::CrawlerSelector;
use pushkind_common::models::zmq::dantes::ZMQMessage;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use pushkind_common::zmq::send_zmq_message;
use serde::Deserialize;
use tera::Tera;

use crate::models::config::ServerConfig;
use crate::repository::{CrawlerReader, DieselRepository, ProductListQuery, ProductReader};

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
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

    let page = params.page.unwrap_or(1);

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &server_config.auth_service_url,
    );

    let product_repo = DieselRepository::new(&pool);
    let crawler_repo = DieselRepository::new(&pool);

    let crawler_id = crawler_id.into_inner();

    let crawler = match crawler_repo.get_crawler_by_id(crawler_id) {
        Ok(Some(crawler)) if crawler.hub_id == user.hub_id => crawler,
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Парсер не существует").send();
            return redirect("/");
        }
    };

    let products = match product_repo.list_products(
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

    context.insert("products", &products);
    context.insert("crawler", &crawler);

    render_template(&tera, "products/index.html", &context)
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

    let repo = DieselRepository::new(&pool);

    let crawler = match repo.get_crawler_by_id(crawler_id) {
        Ok(Some(crawler)) if crawler.hub_id == user.hub_id => crawler,
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Парсер не существует").send();
            return redirect("/");
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

#[post("/crawler/{crawler_id}/update")]
pub async fn update_crawler_prices(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    }

    let crawler_id = crawler_id.into_inner();

    let crawler_repo = DieselRepository::new(&pool);
    let product_repo = DieselRepository::new(&pool);

    let crawler = match crawler_repo.get_crawler_by_id(crawler_id) {
        Ok(Some(crawler)) if crawler.hub_id == user.hub_id => crawler,
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return HttpResponse::InternalServerError().finish();
        }
        _ => {
            FlashMessage::error("Парсер не существует").send();
            return redirect("/");
        }
    };

    let crawler_products =
        match product_repo.list_products(ProductListQuery::default().crawler(crawler_id)) {
            Ok((_, crawler_products)) => crawler_products,
            Err(e) => {
                log::error!("Failed to get products: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    let message = ZMQMessage::Crawler(CrawlerSelector::SelectorProducts((
        crawler.selector,
        crawler_products.into_iter().map(|p| p.url).collect(),
    )));
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
