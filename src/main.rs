use actix_files::Files;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use chrono::prelude::*;
use dustlog::HTTPRequestLog;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Write};

#[derive(Serialize, Deserialize)]
struct EndpointNames {
    health_check: &'static str,
}

impl EndpointNames {
    const fn new(endpoint: &'static str) -> Self {
        EndpointNames {
            health_check: endpoint,
        }
    }
}

const API_ENDPOINTS: EndpointNames = EndpointNames::new("/api/v1/health_check");

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    write_api_endpoints_to_json_file()?;

    HttpServer::new(|| {
        App::new()
            .route(&API_ENDPOINTS.health_check, web::get().to(api_health_check))
            .service(Files::new("/", get_env_var("DUST_CHAT_PATH")).index_file("index.html"))
    })
    .bind((
        "127.0.0.1",
        get_env_var("DUST_SERVER_PORT").parse::<u16>().unwrap(),
    ))?
    .run()
    .await
}

async fn api_health_check(req: HttpRequest) -> impl Responder {
    let log = HTTPRequestLog {
        timestamp: Utc::now(),
        requester_ip_address: req
            .connection_info()
            .realip_remote_addr()
            .unwrap()
            .to_owned(),
        restful_method: req.method().to_string(),
        api_called: req.path().to_owned(),
    };

    println!("{}", log.as_log_str());

    HttpResponse::Ok().body("Ok")
}

fn write_api_endpoints_to_json_file() -> std::io::Result<()> {
    let mut f = File::create(format!(
        "{}{}",
        get_env_var("DUST_CHAT_PATH"),
        "endpoints_v1.json"
    ))?;

    f.write_all(serde_json::to_string(&API_ENDPOINTS).unwrap().as_bytes())?;

    Ok(())
}

fn get_env_var(desired_env_var: &str) -> String {
    match env::var(desired_env_var) {
        Ok(v) => v,
        Err(e) => panic!("${} is not set ({})", desired_env_var, e),
    }
}
