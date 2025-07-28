use actix_web::HttpResponse;
use lazy_static::lazy_static;
use log;
use tera::{Context, Tera};

pub mod main;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {e}");
                ::std::process::exit(1);
            }
        }
    };
}

fn render_template(template: &str, context: &Context) -> HttpResponse {
    HttpResponse::Ok().body(TEMPLATES.render(template, context).unwrap_or_else(|e| {
        log::error!("Failed to render template '{template}': {e}");
        String::new()
    }))
}
