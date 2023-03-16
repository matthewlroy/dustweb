use actix_files::Files;
use actix_web::{
    web::{self, Bytes},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use chrono::prelude::*;
use dustcfg::{get_env_var, API_ENDPOINTS};
use dustlog::{write_to_server_log, HTTPRequestLog, LogLevel};
use email_address::*;
use serde::{Deserialize, Serialize};
use std::str;

// max payload size is 256 Kb (1024 scale)
// const MAX_PAYLOAD_SIZE: usize = 262_144;

#[derive(Serialize, Deserialize)]
struct CreateUserSchema {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct BadResponseSchema {
    error_field: &'static str,
    error_message: &'static str,
}

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
            match serde_json::from_str::<CreateUserSchema>(&seralized_utf8_str) {
                Ok(create_user_obj) => {
                    // 0: Log the request to server
                    capture_request_log(
                        LogLevel::INFO,
                        req,
                        Some("* * * * USER CREDS REDACTED * * * *".to_owned()),
                    );

                    // 1: Validate input
                    if EmailAddress::is_valid(&create_user_obj.email) == false {
                        let invalid_email_response = BadResponseSchema {
                            error_field: "email",
                            error_message: "Please enter a valid email address.",
                        };
                        return HttpResponse::BadRequest().json(web::Json(&invalid_email_response));
                    }

                    if create_user_obj.password.len() < 8 || create_user_obj.password.len() > 255 {
                        let invalid_password = BadResponseSchema {
                            error_field: "password",
                            error_message:
                                "Please enter a valid password of at least 8 characters.",
                        };
                        return HttpResponse::BadRequest().json(web::Json(&invalid_password));
                    }

                    // TODO: 2: Sanitize email, hash + salt the password

                    // TODO: 4: Send to middleware/dustDb

                    // TODO: 5: Bring response back to client

                    return HttpResponse::Ok().finish();
                }
                // Cannot parse the request into the CreateUserSchema, bad request!
                Err(e) => {
                    capture_request_log(LogLevel::ERROR, req, Some(e.to_string()));
                    return HttpResponse::BadRequest().finish();
                }
            }
        }
        Err(e) => {
            // Cannot deserialize the bytes, bad request!
            capture_request_log(LogLevel::ERROR, req, Some(e.to_string()));
            HttpResponse::BadRequest().finish()
        }
    }
}

// fn check_payload_size(req: &HttpRequest, bytes: &Bytes) -> Result<(), HttpResponse> {
//     if bytes.len() > MAX_PAYLOAD_SIZE {
//         let err_string: String = format!("Request payload exceeds {}", MAX_PAYLOAD_SIZE);
//         let payload_too_large_resp = BadResponseSchema {
//             error_field: "server",
//             error_message: "BLAH",
//         };

//         capture_request_log(
//             LogLevel::ERROR,
//             req.to_owned(),
//             Some(err_string),
//         );

//         Err(HttpResponse::PayloadTooLarge().json(web::Json(&payload_too_large_resp)))
//     } else {
//         Ok(())
//     }
// }

async fn api_health_check(req: HttpRequest) -> impl Responder {
    capture_request_log(LogLevel::INFO, req, None);
    HttpResponse::Ok().finish()
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
