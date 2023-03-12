use actix_files::Files;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use chrono::prelude::*;
use dustcfg::{get_env_var, API_ENDPOINTS};
use dustlog::{HTTPRequestLog, LogLevel};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route(API_ENDPOINTS.create_user, web::post().to(api_create_user))
            .route(API_ENDPOINTS.health_check, web::get().to(api_health_check))
            .service(Files::new("/", get_env_var("DUST_CHAT_PATH")).index_file("index.html"))
    })
    .bind((
        "127.0.0.1",
        get_env_var("DUST_SERVER_PORT").parse::<u16>().unwrap(),
    ))?
    .run()
    .await
}

async fn api_create_user(req: HttpRequest) -> impl Responder {
    write_request_to_log(req);
    HttpResponse::Ok().body("posted")
}

async fn api_health_check(req: HttpRequest) -> impl Responder {
    write_request_to_log(req);
    HttpResponse::Ok().body("Ok")
}

fn write_request_to_log(req: HttpRequest) {
    let _ = HTTPRequestLog {
        log_level: LogLevel::INFO,
        timestamp: Utc::now(),
        requester_ip_address: req
            .connection_info()
            .realip_remote_addr()
            .unwrap()
            .to_owned(),
        restful_method: req.method().to_string(),
        api_called: req.path().to_owned(),
    }
    .write_to_server_log();
}