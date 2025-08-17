use actix_multipart::form::MultipartForm;
use actix_web::http::header;
use actix_web::{get, post, web, HttpResponse, Responder};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::send_zmq_message;
use tera::Tera;

use crate::forms::benchmarks::{
    AddBenchmarkForm, AssociateForm, UnassociateForm, UploadBenchmarksForm,
};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::benchmarks::{
    add_benchmark as add_benchmark_service,
    create_benchmark_product as create_benchmark_product_service,
    delete_benchmark_product as delete_benchmark_product_service,
    match_benchmark as match_benchmark_service,
    show_benchmark as show_benchmark_service,
    show_benchmarks as show_benchmarks_service,
    update_benchmark_prices as update_benchmark_prices_service,
    upload_benchmarks as upload_benchmarks_service,
};
use crate::services::errors::ServiceError;

#[get("/benchmarks")]
pub async fn show_benchmarks(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match show_benchmarks_service(repo.get_ref(), &user) {
        Ok(benchmarks) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "benchmarks",
                &server_config.auth_service_url,
            );

            context.insert("benchmarks", &benchmarks);

            render_template(&tera, "benchmarks/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Found()
            .append_header((header::LOCATION, "/na"))
            .finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/benchmark/{benchmark_id}")]
pub async fn show_benchmark(
    benchmark_id: web::Path<i32>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match show_benchmark_service(repo.get_ref(), &user, benchmark_id.into_inner()) {
        Ok((benchmark, products, distances)) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "benchmarks",
                &server_config.auth_service_url,
            );
            context.insert("benchmark", &benchmark);
            context.insert("crawler_products", &products);
            context.insert("distances", &distances);
            render_template(&tera, "benchmarks/benchmark.html", &context)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Found()
            .append_header((header::LOCATION, "/na"))
            .finish(),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
            redirect("/benchmarks")
        }
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/benchmark/add")]
pub async fn add_benchmark(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddBenchmarkForm>,
) -> impl Responder {
    match add_benchmark_service(repo.get_ref(), &user, form) {
        Ok(true) => FlashMessage::success("Бенчмарк добавлен.".to_string()).send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении бенчмарка").send(),
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish()
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmark/{benchmark_id}/match")]
pub async fn match_benchmark(
    benchmark_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    match match_benchmark_service(
        repo.get_ref(),
        &user,
        benchmark_id.into_inner(),
        |msg| send_zmq_message(msg, &server_config.zmq_address).map_err(|_| ()),
    ) {
        Ok(true) => FlashMessage::success("Обработка запущена").send(),
        Ok(false) => FlashMessage::error("Не удалось начать обработку.").send(),
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish()
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmarks/upload")]
pub async fn upload_benchmarks(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadBenchmarksForm>,
) -> impl Responder {
    match upload_benchmarks_service(repo.get_ref(), &user, &mut form) {
        Ok(true) => FlashMessage::success("Бенчмарки добавлены.".to_string()).send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении бенчмарков").send(),
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish()
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmark/{benchmark_id}/update")]
pub async fn update_benchmark_prices(
    benchmark_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    match update_benchmark_prices_service(
        repo.get_ref(),
        &user,
        benchmark_id.into_inner(),
        |msg| send_zmq_message(msg, &server_config.zmq_address).map_err(|_| ()),
    ) {
        Ok(results) => {
            for (selector, sent) in results {
                if sent {
                    FlashMessage::success(format!(
                        "Обработка запущена для {}",
                        selector
                    ))
                    .send();
                } else {
                    FlashMessage::error(format!(
                        "Не удалось начать обработку для {}",
                        selector
                    ))
                    .send();
                }
            }
        }
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish();
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmark/unassociate")]
pub async fn delete_benchmark_product(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<UnassociateForm>,
) -> impl Responder {
    match delete_benchmark_product_service(repo.get_ref(), &user, form) {
        Ok(true) => FlashMessage::success("Мэтчинг удален.").send(),
        Ok(false) => FlashMessage::error("Ошибка при удалении мэтчинга").send(),
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish();
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк или товар не существует").send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmark/associate")]
pub async fn create_benchmark_product(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AssociateForm>,
) -> impl Responder {
    match create_benchmark_product_service(repo.get_ref(), &user, form) {
        Ok(true) => FlashMessage::success("Мэтчинг добавлен.").send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении мэтчинга").send(),
        Err(ServiceError::Unauthorized) => {
            return HttpResponse::Found()
                .append_header((header::LOCATION, "/na"))
                .finish();
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк или товар не существует").send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}
