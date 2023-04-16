#![feature(result_option_inspect)]

use actix::*;
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use expanduser::expanduser;

use actix_web::rt::time::sleep;
use reqwest::Client;
use std::time::Duration;

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
            metadata_url: "http://localhost:9999".to_string(),
            port: 8000,
            cache_server_url: "http://localhost:8000".to_string(),
        }
    }
}

async fn ping_metadata_server_coroutine(metadata_url: String, cache_server_url: String) {
    loop {
        let client = Client::new();
        match client
            .post(metadata_url.clone() + "/ping")
            .header("User-Agent", &cache_server_url)
            .send()
            .await
        {
            Ok(_) => log::info!("Send ping to metadata server."),
            Err(err) => log::warn!("Failed to send ping to metadata server {:?}", err),
        }
        sleep(Duration::from_secs(30)).await;
    }
}

fn goodbye_metadata_server(metadata_url: String, hostname: String) {
    match ureq::post(&(metadata_url.clone() + "/goodbye"))
        .set("User-Agent", &hostname)
        .call()
    {
        Ok(_) => log::info!("Send goodbye to metadata server."),
        Err(err) => log::warn!("Failed to send goodbye to metadata server {:?}", err),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cfg: dto::WebServiceConfig =
        confy::load_path("cfg.yml").expect("Failed to load config from disk");

    let cache = data_cache::DataCache::new(
        cfg.basedir,
        cfg.metadata_url.clone(),
        cfg.cache_server_url.clone(),
    );
    let handle = actix_rt::spawn(ping_metadata_server_coroutine(
        cfg.metadata_url.clone(),
        cfg.cache_server_url.clone(),
    ));
    let cache = cache.start();

    log::info!("starting HTTP server at http://localhost:8000");
    let res = HttpServer::new(move || {
        let cors = Cors::permissive();
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
            .route(
                "/v1/get/current_cache",
                web::get().to(requesthandler::get_current_cache),
            )
            .wrap(Logger::default())
            .wrap(cors)
    })
    .workers(2)
    .bind(("127.0.0.1", cfg.port as u16))?
    .run()
    .await;
    handle.abort();
    goodbye_metadata_server(cfg.metadata_url, cfg.cache_server_url);
    res
}
