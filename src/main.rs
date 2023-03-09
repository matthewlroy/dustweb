use actix_files::Files;
use actix_web::{App, HttpServer};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(Files::new("/", dust_get_env_var("DUST_CHAT_PATH")).index_file("index.html"))
    })
    .bind((
        "127.0.0.1",
        dust_get_env_var("DUST_SERVER_PORT").parse::<u16>().unwrap(),
    ))?
    .run()
    .await
}

fn dust_get_env_var(desired_env_var: &str) -> String {
    match env::var(desired_env_var) {
        Ok(v) => v,
        Err(e) => panic!("${} is not set ({})", desired_env_var, e),
    }
}
