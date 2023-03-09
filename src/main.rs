use actix_files::Files;
use actix_web::{middleware::Logger, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(Files::new("/", "/dust/dustchat/").index_file("index.html"))
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await
}
