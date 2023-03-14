use actix_files::Files;
use actix_web::{
    web::{self, Bytes},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use chrono::prelude::*;
use dustcfg::{get_env_var, API_ENDPOINTS};
use dustlog::{write_to_server_log, HTTPRequestLog, LogLevel};
use std::str;

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

async fn api_create_user(req: HttpRequest, bytes: Bytes) -> impl Responder {
    match str::from_utf8(&bytes.to_vec()) {
        Ok(seralized_utf8_str) => {
            capture_request_log(
                LogLevel::INFO,
                req,
                Some("* * * * USER CREDS REDACTED * * * *".to_owned()),
            );

            // TODO: Make sure we have an email and password here from the input
            // TODO: Hash + Salt the password
            // TODO: Send to middleware/dustDb
            // TODO: Bring response back to client

            HttpResponse::Ok().body("posted")
        }
        Err(e) => {
            capture_request_log(LogLevel::ERROR, req, Some(e.to_string()));
            HttpResponse::BadRequest().body("error")
        }
    }
}

async fn api_health_check(req: HttpRequest) -> impl Responder {
    capture_request_log(LogLevel::INFO, req, None);
    HttpResponse::Ok().body("Ok")
}

fn capture_request_log(level: LogLevel, req: HttpRequest, request_body_utf8_str: Option<String>) {
    let _ = write_to_server_log(
        &HTTPRequestLog {
            log_level: level,
            timestamp: Utc::now(),
            requester_ip_address: req
                .connection_info()
                .realip_remote_addr()
                .unwrap()
                .to_owned(),
            restful_method: req.method().to_string(),
            api_called: req.path().to_owned(),
            request_body_utf8_str: request_body_utf8_str,
        }
        .as_log_str(),
    );
}
