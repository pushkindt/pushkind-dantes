use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::repository::DieselRepository;
use crate::services::api::{ApiV1ProductsQueryParams, api_v1_products as api_v1_products_service};
use crate::services::errors::ServiceError;

#[get("/v1/products")]
pub async fn api_v1_products(
    params: web::Query<ApiV1ProductsQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_v1_products_service(repo.get_ref(), params.into_inner(), &user) {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Internal) => HttpResponse::InternalServerError().finish(),
    }
}
