use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use tera::Tera;

use crate::forms::categories::{
    AddCategoryForm, AddCategoryFormPayload, ClearProductCategoryForm,
    ClearProductCategoryFormPayload, DeleteCategoryForm, DeleteCategoryFormPayload,
    SetProductCategoryForm, SetProductCategoryFormPayload, UpdateCategoryForm,
    UpdateCategoryFormPayload,
};
use crate::repository::DieselRepository;
use crate::services::ServiceError;
use crate::services::categories::{
    add_category as add_category_service,
    clear_product_category_manual as clear_product_category_service,
    delete_category as delete_category_service,
    match_product_categories as match_product_categories_service,
    set_product_category_manual as set_product_category_service,
    show_categories as show_categories_service, update_category as update_category_service,
};

#[get("/categories")]
pub async fn show_categories(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match show_categories_service(&user, repo.get_ref()) {
        Ok(categories) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "categories",
                &server_config.auth_service_url,
            );
            context.insert("categories", &categories);
            render_template(&tera, "categories/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/categories")
        }
        Err(err) => {
            log::error!("Failed to render categories page: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/categories")]
pub async fn add_category(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddCategoryForm>,
) -> impl Responder {
    let payload: AddCategoryFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect("/categories");
        }
    };

    match add_category_service(payload, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Категория добавлена.").send(),
        Ok(false) => FlashMessage::error("Ошибка при добавлении категории.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => FlashMessage::error("Категория не найдена.").send(),
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to add category: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/categories")
}

#[post("/categories/{category_id}/update")]
pub async fn update_category(
    category_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<UpdateCategoryForm>,
) -> impl Responder {
    let mut payload: UpdateCategoryFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect("/categories");
        }
    };

    payload.category_id = match category_id.into_inner().try_into() {
        Ok(id) => id,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect("/categories");
        }
    };

    match update_category_service(payload, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Категория обновлена.").send(),
        Ok(false) => FlashMessage::error("Ошибка при обновлении категории.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => FlashMessage::error("Категория не найдена.").send(),
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to update category: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/categories")
}

#[post("/categories/{category_id}/delete")]
pub async fn delete_category(
    category_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<DeleteCategoryForm>,
) -> impl Responder {
    let mut payload: DeleteCategoryFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect("/categories");
        }
    };

    payload.category_id = match category_id.into_inner().try_into() {
        Ok(id) => id,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect("/categories");
        }
    };

    match delete_category_service(payload, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Категория удалена.").send(),
        Ok(false) => FlashMessage::error("Ошибка при удалении категории.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => FlashMessage::error("Категория не найдена.").send(),
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to delete category: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/categories")
}

#[post("/products/{product_id}/category")]
pub async fn set_product_category_manual(
    request: HttpRequest,
    product_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<SetProductCategoryForm>,
) -> impl Responder {
    let redirect_to = request
        .headers()
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("/");

    let mut payload: SetProductCategoryFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect(redirect_to);
        }
    };

    payload.product_id = match product_id.into_inner().try_into() {
        Ok(id) => id,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect(redirect_to);
        }
    };

    match set_product_category_service(payload, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Категория товара обновлена вручную.").send(),
        Ok(false) => FlashMessage::error("Ошибка при обновлении категории товара.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Товар или категория не найдены.").send()
        }
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to set manual product category: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect(redirect_to)
}

#[post("/products/{product_id}/category/clear")]
pub async fn clear_product_category_manual(
    request: HttpRequest,
    product_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<ClearProductCategoryForm>,
) -> impl Responder {
    let redirect_to = request
        .headers()
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("/");

    let mut payload: ClearProductCategoryFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect(redirect_to);
        }
    };

    payload.product_id = match product_id.into_inner().try_into() {
        Ok(id) => id,
        Err(e) => {
            FlashMessage::error(e.to_string()).send();
            return redirect(redirect_to);
        }
    };

    match clear_product_category_service(payload, &user, repo.get_ref()) {
        Ok(true) => FlashMessage::success("Ручная категория очищена.").send(),
        Ok(false) => FlashMessage::error("Ошибка при очистке ручной категории.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => FlashMessage::error("Товар не найден.").send(),
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to clear manual product category: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect(redirect_to)
}

#[post("/categories/match-products")]
pub async fn match_product_categories(
    user: AuthenticatedUser,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match match_product_categories_service(&user, zmq_sender.get_ref().as_ref()).await {
        Ok(true) => FlashMessage::success("Матчинг категорий по товарам запущен.").send(),
        Ok(false) => FlashMessage::error("Не удалось запустить матчинг категорий.").send(),
        Err(ServiceError::Unauthorized) => return redirect("/na"),
        Err(ServiceError::NotFound) => FlashMessage::error("Ресурс не найден.").send(),
        Err(ServiceError::Form(message)) => FlashMessage::error(message).send(),
        Err(ServiceError::Internal) => return HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to enqueue product category matching: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    redirect("/categories")
}
