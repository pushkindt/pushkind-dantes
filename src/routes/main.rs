use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::IncomingFlashMessages;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::repository::DieselRepository;
use crate::services::errors::ServiceError;
use crate::services::main::show_index as show_index_service;

#[get("/")]
pub async fn index(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match show_index_service(repo.get_ref(), &user) {
        Ok(crawlers) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );

            context.insert("crawlers", &crawlers);

            render_template(&tera, "main/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}
