use std::sync::Arc;

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use tera::Tera;

use crate::repository::DieselRepository;
use crate::services::errors::ServiceError;
use crate::services::products::{
    crawl_crawler as crawl_crawler_service, show_products as show_products_service,
    update_crawler_prices as update_crawler_prices_service,
};

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
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let page = params.page.unwrap_or(1);
    match show_products_service(repo.get_ref(), &user, crawler_id.into_inner(), page) {
        Ok((crawler, products)) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("products", &products);
            context.insert("crawler", &crawler);
            render_template(&tera, "products/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Парсер не существует").send();
            redirect("/")
        }
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/crawler/{crawler_id}/crawl")]
pub async fn crawl_crawler(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match crawl_crawler_service(
        repo.get_ref(),
        &user,
        crawler_id.into_inner(),
        async |msg| zmq_sender.send_json(msg).await.map_err(|_| ()),
    )
    .await
    {
        Ok(true) => {
            FlashMessage::success("Обработка запущена").send();
            redirect("/")
        }
        Ok(false) => {
            FlashMessage::error("Не удалось начать обработку.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Парсер не существует").send();
            redirect("/")
        }
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/crawler/{crawler_id}/update")]
pub async fn update_crawler_prices(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match update_crawler_prices_service(
        repo.get_ref(),
        &user,
        crawler_id.into_inner(),
        async |msg| zmq_sender.send_json(msg).await.map_err(|_| ()),
    )
    .await
    {
        Ok(true) => {
            FlashMessage::success("Обработка запущена").send();
            redirect("/")
        }
        Ok(false) => {
            FlashMessage::error("Не удалось начать обработку.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Парсер не существует").send();
            redirect("/")
        }
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}
