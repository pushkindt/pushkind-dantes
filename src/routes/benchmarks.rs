use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use pushkind_common::zmq::send_zmq_message;
use serde::Deserialize;
use tera::Context;
use validator::Validate;

use crate::domain::benchmark::NewBenchmark;
use crate::domain::crawler::Crawler;
use crate::domain::product::Product;
use crate::forms::benchmarks::{AddBenchmarkForm, UploadBenchmarksForm};
use crate::models::config::ServerConfig;
use crate::repository::benchmark::DieselBenchmarkRepository;
use crate::repository::crawler::DieselCrawlerRepository;
use crate::repository::product::DieselProductRepository;
use crate::repository::{BenchmarkListQuery, ProductListQuery};
use crate::repository::{BenchmarkReader, BenchmarkWriter, CrawlerReader, ProductReader};
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

#[get("/benchmark/{benchmark_id}")]
pub async fn show_benchmark(
    benchmark_id: web::Path<i32>,
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
    context.insert("current_page", "benchmarks");
    context.insert("home_url", &server_config.auth_service_url);

    let benchmark_id = benchmark_id.into_inner();

    let benchmark_repo = DieselBenchmarkRepository::new(&pool);

    let benchmark = match benchmark_repo.get_by_id(benchmark_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => {
            FlashMessage::error("Бенчмарк не существует.").send();
            return redirect("/benchmarks");
        }
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let crawler_repo = DieselCrawlerRepository::new(&pool);

    let crawlers = match crawler_repo.list(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let product_repo = DieselProductRepository::new(&pool);

    let mut products: Vec<(Crawler, Vec<Product>)> = vec![];

    for crawler in crawlers {
        let crawler_products = match product_repo.list(
            ProductListQuery::default()
                .benchmark(benchmark_id)
                .crawler(crawler.id),
        ) {
            Ok((_total, products)) => products,
            Err(e) => {
                log::error!("Failed to list products: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };
        products.push((crawler, crawler_products));
    }

    let distances = match product_repo.list_distances(benchmark_id) {
        Ok(distances) => distances,
        Err(e) => {
            log::error!("Failed to list distances: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("benchmark", &benchmark);
    context.insert("crawler_products", &products);
    context.insert("distances", &distances);

    render_template("benchmarks/benchmark.html", &context)
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

#[post("/benchmark/{benchmark_id}/process")]
pub async fn process_benchmark(
    benchmark_id: web::Path<i32>,
    user: AuthenticatedUser,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "parser", Some("/na")) {
        return response;
    };

    let benchmark_id = format!("{}", benchmark_id.into_inner());
    match send_zmq_message(benchmark_id.as_bytes(), &server_config.zmq_address) {
        Ok(_) => {
            FlashMessage::success("Обработка запущена").send();
        }
        Err(e) => {
            log::error!("Failed to send ZMQ message: {e}");
            FlashMessage::error("Не удалось начать обработку.").send();
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
