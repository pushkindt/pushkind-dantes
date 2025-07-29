use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use serde::Deserialize;
use tera::Context;
use validator::Validate;

use crate::domain::benchmark::NewBenchmark;
use crate::forms::benchmarks::{AddBenchmarkForm, UploadBenchmarksForm};
use crate::repository::BenchmarkListQuery;
use crate::repository::benchmark::DieselBenchmarkRepository;
use crate::repository::{BenchmarkReader, BenchmarkWriter};
use crate::routes::render_template;

#[derive(Deserialize)]
struct BenchmarkQueryParams {
    page: Option<usize>,
}

#[get("/benchmarks")]
pub async fn show_benchmarks(
    params: web::Query<BenchmarkQueryParams>,
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
    context.insert("current_page", "benchmarks");
    context.insert("home_url", &server_config.auth_service_url);

    let repo = DieselBenchmarkRepository::new(&pool);

    let benchmarks = match repo
        .list(BenchmarkListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE))
    {
        Ok((total, benchmarks)) => {
            Paginated::new(benchmarks, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
        }
        Err(e) => {
            log::error!("Failed to list benchmarks: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("benchmarks", &benchmarks);

    render_template("benchmarks/index.html", &context)
}

#[post("/benchmark/add")]
pub async fn add_benchmark(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddBenchmarkForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    };

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        FlashMessage::error("Ошибка валидации формы").send();
        return redirect("/benchmarks");
    }

    let new_benchmark: NewBenchmark = form.into();

    let repo = DieselBenchmarkRepository::new(&pool);
    match repo.create(&[new_benchmark]) {
        Ok(_) => {
            FlashMessage::success("Бенчмарк добавлен.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to add a benchmark: {err}");
            FlashMessage::error("Ошибка при добавлении бенчмарка").send();
        }
    }
    redirect("/benchmarks")
}

#[post("/benchmarks/upload")]
pub async fn upload_benchmarks(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    MultipartForm(mut form): MultipartForm<UploadBenchmarksForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    };

    let benchmark_repo = DieselBenchmarkRepository::new(&pool);

    let benchmarks = match form.parse(user.hub_id) {
        Ok(benchmarks) => benchmarks,
        Err(err) => {
            log::error!("Failed to parse benchmarks: {err}");
            FlashMessage::error("Ошибка при парсинге бенчмарков").send();
            return redirect("/benchmarks");
        }
    };

    match benchmark_repo.create(&benchmarks) {
        Ok(_) => {
            FlashMessage::success("Бенчмарки добавлены.".to_string()).send();
        }
        Err(err) => {
            log::error!("Failed to add benchmarks: {err}");
            FlashMessage::error("Ошибка при добавлении бенчмарков").send();
        }
    }

    redirect("/benchmarks")
}
