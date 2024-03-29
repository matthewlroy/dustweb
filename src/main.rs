use actix_files::Files;
use actix_web::{
    web::{self, Bytes},
    App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use chrono::prelude::*;
use dustcfg::{get_env_var, API_ENDPOINTS};
use dustlog::{write_to_log, HTTPRequestLog, HTTPResponseLog, LogLevel};
use dustmw::{dust_db_create_user, dust_db_health_check, CreateUserSchema};
use email_address::*;
use pwhash::bcrypt;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::str;

// Max payload size is 128Kb (1024 scale => 131,072 bytes)
const MAX_INCOMING_PAYLOAD_SIZE: usize = 131_072;

#[derive(Serialize, Deserialize)]
struct ResponseBodySchema {
    error_field: String,
    error_message: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(
        "{}",
        format!(
            "dustweb successfully started, listening on: {}:{}",
            get_env_var("DUST_SERVER_ADDR"),
            get_env_var("DUST_SERVER_PORT")
        )
    );

    HttpServer::new(|| {
        App::new()
            .route(API_ENDPOINTS.create_user, web::post().to(api_create_user))
            .route(API_ENDPOINTS.health_check, web::get().to(api_health_check))
            .service(Files::new("/", get_env_var("DUST_CHAT_PATH")).index_file("index.html"))
    })
    .bind((
        get_env_var("DUST_SERVER_ADDR"),
        get_env_var("DUST_SERVER_PORT").parse::<u16>().unwrap(),
    ))?
    .run()
    .await
}

async fn api_create_user(req: HttpRequest, bytes: Bytes) -> impl Responder {
    match request_payload_handler(&req, &bytes) {
        Ok(_) => {
            match str::from_utf8(&bytes.to_vec()) {
                Ok(seralized_utf8_str) => {
                    match serde_json::from_str::<CreateUserSchema>(&seralized_utf8_str) {
                        Ok(mut create_user_obj) => {
                            // 0: Log the request to server
                            capture_request_log(
                                LogLevel::INFO,
                                &req,
                                Some(bytes.len()),
                                Some("User credentials redacted . . .".to_owned()),
                            );

                            // 1: Validate input

                            // 1a: Validate email!
                            if EmailAddress::is_valid(&create_user_obj.email) == false {
                                let invalid_email_response = ResponseBodySchema {
                                    error_field: "email".to_owned(),
                                    error_message: "Please enter a valid email address.".to_owned(),
                                };

                                return response_handler(
                                    HttpResponse::BadRequest(),
                                    Some(invalid_email_response),
                                );
                            }

                            // 1b: Validate password!
                            if create_user_obj.password.len() < 8
                                || create_user_obj.password.len() > 255
                            {
                                let invalid_password = ResponseBodySchema {
                                    error_field: "password".to_owned(),
                                    error_message:
                                        "Please enter a valid password of at least 8 characters."
                                            .to_owned(),
                                };

                                return response_handler(
                                    HttpResponse::BadRequest(),
                                    Some(invalid_password),
                                );
                            }

                            // 2: Sanitize email, hash password
                            create_user_obj.email = create_user_obj.email.trim().to_lowercase();
                            create_user_obj.password =
                                bcrypt::hash(create_user_obj.password).unwrap();

                            // 3: Send to middleware/dustDb
                            match dust_db_create_user(create_user_obj).await {
                                // TODO: Navigate user? Use the uuidv4? Next steps here!
                                Ok(_) => response_handler(HttpResponse::Ok(), None),
                                // Something went wrong when creating the user
                                Err(e) => match e.kind() {
                                    // User already exists!
                                    ErrorKind::AlreadyExists => {
                                        let db_error_resp = ResponseBodySchema {
                                            error_field: "email".to_owned(),
                                            error_message: e.to_string(),
                                        };

                                        response_handler(
                                            HttpResponse::Conflict(),
                                            Some(db_error_resp),
                                        )
                                    }

                                    // Generic error catch-all
                                    _ => {
                                        let db_error_resp = ResponseBodySchema {
                                            error_field: "server".to_owned(),
                                            error_message: e.to_string(),
                                        };

                                        response_handler(
                                            HttpResponse::InternalServerError(),
                                            Some(db_error_resp),
                                        )
                                    }
                                },
                            }
                        }
                        // Cannot parse the request into the CreateUserSchema, bad request!
                        Err(e) => {
                            let bad_schema_err_resp = ResponseBodySchema {
                                error_field: "server".to_owned(),
                                error_message: "Error occurred parsing the request into JSON"
                                    .to_string(),
                            };

                            capture_request_log(
                                LogLevel::ERROR,
                                &req,
                                Some(bytes.len()),
                                Some(e.to_string()),
                            );

                            response_handler(HttpResponse::BadRequest(), Some(bad_schema_err_resp))
                        }
                    }
                }
                Err(e) => {
                    // Cannot deserialize the bytes, bad request!
                    let deserialize_bytes_err_resp = ResponseBodySchema {
                        error_field: "server".to_owned(),
                        error_message: "Error occurred deserializing the requested payload"
                            .to_string(),
                    };

                    capture_request_log(
                        LogLevel::ERROR,
                        &req,
                        Some(bytes.len()),
                        Some(e.to_string()),
                    );

                    response_handler(HttpResponse::BadRequest(), Some(deserialize_bytes_err_resp))
                }
            }
        }
        Err(payload_err) => payload_err,
    }
}

async fn api_health_check(req: HttpRequest) -> impl Responder {
    capture_request_log(LogLevel::INFO, &req, None, None);

    match dust_db_health_check().await {
        Ok(_) => response_handler(HttpResponse::Ok(), None),
        // Couldn't connect to our DB, error out!
        Err(e) => {
            let no_db_conn_resp = ResponseBodySchema {
                error_field: "server".to_owned(),
                error_message: e.to_string(),
            };

            response_handler(HttpResponse::InternalServerError(), Some(no_db_conn_resp))
        }
    }
}

fn request_payload_handler(req: &HttpRequest, bytes: &Bytes) -> Result<(), HttpResponse> {
    if bytes.len() > MAX_INCOMING_PAYLOAD_SIZE {
        let err_string: String = format!(
            "Request payload exceeds {} bytes",
            MAX_INCOMING_PAYLOAD_SIZE
        );

        let payload_too_large_resp = ResponseBodySchema {
            error_field: "server".to_owned(),
            error_message: err_string.clone(),
        };

        capture_request_log(
            LogLevel::ERROR,
            req,
            Some(bytes.len()),
            Some("Payload too large to display . . .".to_owned()),
        );

        Err(response_handler(
            HttpResponse::PayloadTooLarge(),
            Some(payload_too_large_resp),
        ))
    } else {
        Ok(())
    }
}

fn response_handler(
    mut response_builder: HttpResponseBuilder,
    response_body_schema: Option<ResponseBodySchema>,
) -> HttpResponse {
    match response_body_schema {
        Some(response_body_schema) => {
            let response = response_builder.json(web::Json(&response_body_schema));

            capture_response_log(
                &response,
                Some(serde_json::to_string(&response_body_schema).unwrap()),
            );

            response
        }
        None => {
            let response = response_builder.finish();

            capture_response_log(&response, None);

            response
        }
    }
}

fn capture_response_log(res: &HttpResponse, body_as_utf8_str: Option<String>) {
    let log = HTTPResponseLog {
        timestamp: Utc::now(),
        log_level: get_log_level_from_status(&res.status().as_u16()),
        originating_ip_addr: get_env_var("DUST_SERVER_ADDR"),
        response_status_code: res.status().as_u16(),
        body_as_utf8_str,
    };

    match write_to_log(log.as_log_str(), log.get_log_distinction()) {
        Ok(_) => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn capture_request_log(
    log_level: LogLevel,
    req: &HttpRequest,
    payload_size_in_bytes: Option<usize>,
    body_as_utf8_str: Option<String>,
) {
    let log = HTTPRequestLog {
        timestamp: Utc::now(),
        log_level,
        originating_ip_addr: req
            .connection_info()
            .realip_remote_addr()
            .unwrap()
            .to_owned(),
        api: req.path().to_owned(),
        restful_method: req.method().to_string(),
        payload_size_in_bytes,
        body_as_utf8_str,
    };

    match write_to_log(log.as_log_str(), log.get_log_distinction()) {
        Ok(_) => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn get_log_level_from_status(status_code: &u16) -> LogLevel {
    match status_code {
        200 => LogLevel::INFO,
        400 => LogLevel::ERROR,
        409 => LogLevel::ERROR,
        413 => LogLevel::ERROR,
        500 => LogLevel::ERROR,
        _ => LogLevel::INFO,
    }
}
