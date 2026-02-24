use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use tera::Tera;

use crate::forms::import_export::UploadImportForm;
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::categories::show_categories as show_categories_service;
use crate::services::products::{
    crawl_crawler as crawl_crawler_service,
    download_crawler_products as download_crawler_products_service,
    show_products as show_products_service, update_crawler_prices as update_crawler_prices_service,
    upload_crawler_products as upload_crawler_products_service,
};

#[derive(Deserialize)]
struct ProductsQueryParams {
    page: Option<usize>,
}

#[derive(Deserialize)]
struct DownloadQueryParams {
    format: String,
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
    let crawler_id = crawler_id.into_inner();
    match show_products_service(crawler_id, page, &user, repo.get_ref()) {
        Ok((crawler, products)) => {
            let categories = match show_categories_service(&user, repo.get_ref()) {
                Ok(categories) => categories,
                Err(ServiceError::Unauthorized) => return redirect("/na"),
                Err(ServiceError::NotFound) => vec![],
                Err(ServiceError::Form(message)) => {
                    FlashMessage::error(message).send();
                    vec![]
                }
                Err(err) => {
                    log::error!("Failed to load categories for products page: {err}");
                    vec![]
                }
            };
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("products", &products);
            context.insert("crawler", &crawler);
            context.insert("categories", &categories);
            context.insert("show_category_controls", &true);
            render_template(&tera, "products/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Парсер не существует").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/crawler/{crawler_id}"))
        }
        Err(err) => {
            log::error!("Failed to render crawler products: {err}");
            HttpResponse::InternalServerError().finish()
        }
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
        crawler_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.get_ref().as_ref(),
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
        Err(err) => {
            log::error!("Failed to start crawler crawl: {err}");
            HttpResponse::InternalServerError().finish()
        }
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
        crawler_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.get_ref().as_ref(),
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
        Err(err) => {
            log::error!("Failed to update crawler prices: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/crawler/{crawler_id}/products/upload")]
pub async fn upload_crawler_products(
    crawler_id: web::Path<i32>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
    MultipartForm(mut form): MultipartForm<UploadImportForm>,
) -> impl Responder {
    let crawler_id = crawler_id.into_inner();
    match upload_crawler_products_service(crawler_id, &mut form, &user, repo.get_ref()) {
        Ok(report) => {
            if report.errors.is_empty() {
                FlashMessage::success(format!(
                    "Загрузка завершена: создано {}, обновлено {}.",
                    report.created, report.updated
                ))
                .send();
                return redirect(&format!("/crawler/{crawler_id}"));
            }

            let (crawler, products) =
                match show_products_service(crawler_id, 1, &user, repo.get_ref()) {
                    Ok(result) => result,
                    Err(ServiceError::Unauthorized) => return redirect("/na"),
                    Err(ServiceError::NotFound) => {
                        FlashMessage::error("Парсер не существует").send();
                        return redirect("/");
                    }
                    Err(_) => return HttpResponse::InternalServerError().finish(),
                };

            let categories = match show_categories_service(&user, repo.get_ref()) {
                Ok(categories) => categories,
                Err(ServiceError::Unauthorized) => return redirect("/na"),
                Err(_) => vec![],
            };

            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("products", &products);
            context.insert("crawler", &crawler);
            context.insert("categories", &categories);
            context.insert("show_category_controls", &true);
            context.insert("upload_report", &report);
            render_template(&tera, "products/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Парсер не существует").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/crawler/{crawler_id}"))
        }
        Err(err) => {
            log::error!("Failed to upload crawler products: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/crawler/{crawler_id}/products/download")]
pub async fn download_crawler_products(
    crawler_id: web::Path<i32>,
    params: web::Query<DownloadQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match download_crawler_products_service(
        crawler_id.into_inner(),
        &params.format,
        &user,
        repo.get_ref(),
    ) {
        Ok(file) => HttpResponse::Ok()
            .append_header(("Content-Type", file.content_type))
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", file.file_name),
            ))
            .body(file.bytes),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => HttpResponse::BadRequest().body(message),
        Err(err) => {
            log::error!("Failed to download crawler products: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
