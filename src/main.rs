use actix::*;
use actix_web::{middleware::Logger, web, App, HttpServer};
use expanduser::expanduser;

mod bind;
mod data_cache;
mod dto;
mod lod;
mod requesthandler;
mod utils;

impl ::std::default::Default for dto::WebServiceConfig {
    fn default() -> Self {
        Self {
            basedir: expanduser("~/Documents/data/tng/manual_download/")
                .expect("Failed to expand user.")
                .display()
                .to_string(),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cfg: dto::WebServiceConfig =
        confy::load_path("cfg.yml").expect("Failed to load config from disk");

    let cache = data_cache::DataCache::new(cfg.basedir).start();

    log::info!("starting HTTP server at http://localhost:8000");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .route("/rand", web::get().to(requesthandler::get_rand_init))
            .route(
                "/v1/get/splines/{simulation}/{snapshot_id}",
                web::post().to(requesthandler::get_snapshot),
            )
            .route(
                "/v1/get/init/{simulation}/{snapshot_id}",
                web::get().to(requesthandler::get_init),
            )
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
