use std::time::Duration;
use actix::*;
use actix_web::{
    middleware::Logger, web, App, HttpServer, Error, rt::time::{sleep_until, Instant}
};
use actix_web::error::ErrorInternalServerError;
mod data_cache;
mod load_numpy;

async fn get_rand_init(
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<String, Error> {
    sleep_until(Instant::now() + Duration::from_secs(0)).await;
    if let Ok(number) = cache.send(data_cache::RandU{}).await {
        Ok(number.to_string())
    } else {
        Err(ErrorInternalServerError("bad"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let cache = data_cache::DataCache::new().start();
    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .route("/rand", web::get().to(get_rand_init))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
