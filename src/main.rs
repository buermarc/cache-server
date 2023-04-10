use std::time::Duration;
use actix::*;
use actix_web::{
    middleware::Logger, web, App, HttpServer, Error, rt::time::{sleep_until, Instant}
};
use actix_web::error::ErrorInternalServerError;


use serde::{Serialize, Deserialize};

mod bind;
mod data_cache;
mod load_numpy;

#[derive(Serialize, Deserialize)]
struct WebServiceConfig {
    basedir: String
}

impl ::std::default::Default for WebServiceConfig {
    fn default() -> Self { Self { basedir: "~/Documents/data/tng/manual_download/".to_string() } }
}

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

    let cfg: WebServiceConfig = confy::load_path("cfg.yml").expect("Failed to load config from disk");

    let cache = data_cache::DataCache::new(cfg.basedir).start();
    let file_name = "cpp/octree.json";

    log::info!("Loading octree");

    let octree = bind::ffi::load_octree_from_file(file_name.to_string());
    let viewbox = bind::ffi::Viewbox { box_min: bind::ffi::RustVec3::new(2001.0, 2000.0, 2000.0), box_max: bind::ffi::RustVec3::new(2504.0, 2500.0, 2506.0) };
    let values = bind::ffi::get_intersecting_node(octree, viewbox);
    for value in values {
        println!("{}", value);
    }

    log::info!("Finished loading octree");

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
