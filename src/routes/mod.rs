use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use log;
use pushkind_common::{models::auth::AuthenticatedUser, routes::alert_level_to_str};
use tera::{Context, Tera};

pub mod api;
pub mod benchmarks;
pub mod main;
pub mod products;

pub fn render_template(tera: &Tera, template: &str, context: &Context) -> HttpResponse {
    HttpResponse::Ok().body(tera.render(template, context).unwrap_or_else(|e| {
        log::error!("Failed to render template '{template}': {e}");
        String::new()
    }))
}

pub fn base_context(
    flash_messages: &IncomingFlashMessages,
    user: &AuthenticatedUser,
    current_page: &str,
    home_url: &str,
) -> Context {
    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();

    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", user);
    context.insert("current_page", current_page);
    context.insert("home_url", home_url);
    context
}
