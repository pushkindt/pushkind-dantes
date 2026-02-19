use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::api::{ApiV1ProductsQueryParams, api_v1_products as api_v1_products_service};

#[get("/v1/products")]
pub async fn api_v1_products(
    params: web::Query<ApiV1ProductsQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_v1_products_service(params.into_inner(), &user, repo.get_ref()) {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load products via API: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
