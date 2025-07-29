use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role};
use tera::Context;

use crate::repository::benchmark::DieselBenchmarkRepository;
use crate::repository::BenchmarkReader;
use crate::routes::render_template;

#[get("/benchmarks")]
pub async fn show_benchmarks(
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

    let repo = DieselBenchmarkRepository::new(&pool);

    let benchmarks = match repo.list(user.hub_id) {
        Ok(benchmarks) => benchmarks,
        Err(e) => {
            log::error!("Failed to list benchmarks: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("benchmarks", &benchmarks);

    render_template("benchmarks/index.html", &context)
}
