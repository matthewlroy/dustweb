use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::env;

enum DustAPIEndpoints {
    HealthCheck,
}

impl DustAPIEndpoints {
    fn as_str(&self) -> &'static str {
        match self {
            DustAPIEndpoints::HealthCheck => "/api/v1/health_check",
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route(
                DustAPIEndpoints::HealthCheck.as_str(),
                web::get().to(health_check),
            )
            .service(Files::new("/", dust_get_env_var("DUST_CHAT_PATH")).index_file("index.html"))
    })
    .bind((
        "127.0.0.1",
        dust_get_env_var("DUST_SERVER_PORT").parse::<u16>().unwrap(),
    ))?
    .run()
    .await
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Ok")
}

fn dust_get_env_var(desired_env_var: &str) -> String {
    match env::var(desired_env_var) {
        Ok(v) => v,
        Err(e) => panic!("${} is not set ({})", desired_env_var, e),
    }
}
