//! Core library exports for the Dantes service.
//!
//! This crate exposes forms, models, repositories, routes and service layers
//! used by the Dantes web application.
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
use actix_files::Files;
#[cfg(feature = "server")]
use actix_identity::IdentityMiddleware;
#[cfg(feature = "server")]
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
#[cfg(feature = "server")]
use actix_web::cookie::Key;
#[cfg(feature = "server")]
use actix_web::{App, HttpServer, middleware, web};
#[cfg(feature = "server")]
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
#[cfg(feature = "server")]
use pushkind_common::db::establish_connection_pool;
#[cfg(feature = "server")]
use pushkind_common::middleware::RedirectUnauthorized;
#[cfg(feature = "server")]
use pushkind_common::models::config::CommonServerConfig;
#[cfg(feature = "server")]
use pushkind_common::routes::{logout, not_assigned};
#[cfg(feature = "server")]
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
#[cfg(feature = "server")]
use tera::Tera;

#[cfg(feature = "server")]
use crate::models::config::ServerConfig;
#[cfg(feature = "server")]
use crate::repository::DieselRepository;
#[cfg(feature = "server")]
use crate::routes::api::api_v1_products;
#[cfg(feature = "server")]
use crate::routes::benchmarks::{
    add_benchmark, create_benchmark_product, delete_benchmark_product, match_benchmark,
    show_benchmark, show_benchmarks, update_benchmark_prices, upload_benchmarks,
};
#[cfg(feature = "server")]
use crate::routes::main::index;
#[cfg(feature = "server")]
use crate::routes::products::{crawl_crawler, show_products, update_crawler_prices};

#[cfg(feature = "data")]
pub mod domain;
#[cfg(feature = "server")]
pub mod dto;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "data")]
pub mod models;
#[cfg(feature = "server")]
pub mod repository;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "data")]
pub mod schema;
#[cfg(feature = "server")]
pub mod services;

pub mod error_conversions;

#[cfg(feature = "server")]
pub const SERVICE_ACCESS_ROLE: &str = "parser";

#[cfg(feature = "server")]
pub async fn run(server_config: ServerConfig) -> std::io::Result<()> {
    let common_config = CommonServerConfig {
        auth_service_url: server_config.auth_service_url.to_string(),
        secret: server_config.secret.clone(),
    };

    // Start a background ZeroMQ publisher used for crawler processing jobs.
    let zmq_sender = ZmqSender::start(ZmqSenderOptions::push_default(
        &server_config.zmq_crawlers_pub,
    ))
    .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ sender: {e}")))?;

    let zmq_sender = Arc::new(zmq_sender);

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&server_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity, sessions, and flash messages.
    let secret_key = Key::from(server_config.secret.as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = Tera::new(&server_config.templates_dir)
        .map_err(|e| std::io::Error::other(format!("Template parsing error(s): {e}")))?;

    let bind_address = (server_config.address.clone(), server_config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", server_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(web::scope("/api").service(api_v1_products))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(index)
                    .service(crawl_crawler)
                    .service(update_crawler_prices)
                    .service(show_benchmarks)
                    .service(show_benchmark)
                    .service(upload_benchmarks)
                    .service(add_benchmark)
                    .service(match_benchmark)
                    .service(update_benchmark_prices)
                    .service(delete_benchmark_product)
                    .service(create_benchmark_product)
                    .service(show_products)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(server_config.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
    })
    .bind(bind_address)?
    .run()
    .await
}
