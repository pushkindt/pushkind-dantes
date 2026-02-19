use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use tera::Tera;

use crate::forms::benchmarks::{
    AddBenchmarkForm, AssociateForm, UnassociateForm, UploadBenchmarksForm,
};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::benchmarks::{
    add_benchmark as add_benchmark_service,
    create_benchmark_product as create_benchmark_product_service,
    delete_benchmark_product as delete_benchmark_product_service,
    match_benchmark as match_benchmark_service, show_benchmark as show_benchmark_service,
    show_benchmarks as show_benchmarks_service,
    update_benchmark_prices as update_benchmark_prices_service,
    upload_benchmarks as upload_benchmarks_service,
};

#[get("/benchmarks")]
pub async fn show_benchmarks(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match show_benchmarks_service(&user, repo.get_ref()) {
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
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/benchmarks")
        }
        Err(err) => {
            log::error!("Failed to render benchmarks page: {err}");
            HttpResponse::InternalServerError().finish()
        }
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
    match show_benchmark_service(benchmark_id.into_inner(), &user, repo.get_ref()) {
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
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
            redirect("/benchmarks")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/benchmarks")
        }
        Err(err) => {
            log::error!("Failed to render benchmark details: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/benchmark/add")]
pub async fn add_benchmark(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddBenchmarkForm>,
) -> impl Responder {
    match add_benchmark_service(form, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Бенчмарк добавлен.").send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении бенчмарка").send(),
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(err) => {
            log::error!("Failed to add benchmark: {err}");
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
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match match_benchmark_service(
        benchmark_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.get_ref().as_ref(),
    )
    .await
    {
        Ok(true) => FlashMessage::success("Обработка запущена").send(),
        Ok(false) => FlashMessage::error("Не удалось начать обработку.").send(),
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(err) => {
            log::error!("Failed to queue benchmark matching: {err}");
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
    match upload_benchmarks_service(&mut form, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Бенчмарки добавлены.").send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении бенчмарков").send(),
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(err) => {
            log::error!("Failed to upload benchmarks: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/benchmarks")
}

#[post("/benchmark/{benchmark_id}/update")]
pub async fn update_benchmark_prices(
    benchmark_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match update_benchmark_prices_service(
        benchmark_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.get_ref().as_ref(),
    )
    .await
    {
        Ok(results) => {
            for (selector, sent) in results {
                if sent {
                    FlashMessage::success(format!("Обработка запущена для {selector}")).send();
                } else {
                    FlashMessage::error(format!("Не удалось начать обработку для {selector}"))
                        .send();
                }
            }
        }
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(err) => {
            log::error!("Failed to update benchmark prices: {err}");
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
    let benchmark_id = form.benchmark_id;
    match delete_benchmark_product_service(form, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Мэтчинг удален.").send(),
        Ok(false) => FlashMessage::error("Ошибка при удалении мэтчинга").send(),
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк или товар не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(err) => {
            log::error!("Failed to remove benchmark association: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect(&format!("/benchmark/{benchmark_id}"))
}

#[post("/benchmark/associate")]
pub async fn create_benchmark_product(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AssociateForm>,
) -> impl Responder {
    let benchmark_id = form.benchmark_id;
    match create_benchmark_product_service(form, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Мэтчинг добавлен.").send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении мэтчинга").send(),
        Err(ServiceError::Unauthorized) => {
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Бенчмарк или товар не существует").send();
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
        }
        Err(ServiceError::Internal) => {
            return HttpResponse::InternalServerError().finish();
        }
        Err(err) => {
            log::error!("Failed to create benchmark association: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect(&format!("/benchmark/{benchmark_id}"))
}
