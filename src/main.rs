use actix::*;
use actix_web::error::ErrorInternalServerError;
use actix_web::{
    middleware::Logger,
    rt::time::{sleep_until, Instant},
    web, App, Error, HttpServer, Responder,
};
use std::time::Duration;

use expanduser::expanduser;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

mod bind;
mod data_cache;
mod lod;

#[derive(Serialize, Deserialize)]
struct WebServiceConfig {
    basedir: String,
}

#[derive(Deserialize)]
struct ClientState {
    level_of_detail: HashMap<i64, i64>,
    batch_size_lod: i64,
    camera_info: lod::CameraInfo,
}

impl ::std::default::Default for WebServiceConfig {
    fn default() -> Self {
        Self {
            basedir: expanduser("~/Documents/data/tng/manual_download/")
                .expect("Failed to expand user.")
                .display()
                .to_string(),
        }
    }
}

async fn get_rand_init(cache: web::Data<Addr<data_cache::DataCache>>) -> Result<String, Error> {
    sleep_until(Instant::now() + Duration::from_secs(0)).await;
    if let Ok(number) = cache.send(data_cache::RandU {}).await {
        Ok(number.to_string())
    } else {
        Err(ErrorInternalServerError("bad"))
    }
}

async fn get_snapshot(
    params: web::Path<(String, usize)>,
    mut client_state: web::Json<ClientState>,
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<impl Responder, Error> {
    let (simulation, snapshot_id) = (params.0.clone(), params.1);
    let message = data_cache::CacheRequest {
        simulation: simulation.to_string(),
        snapshot_id,
    };
    if let Ok(cache_entry) = cache.send(message).await {
        let cache_entry = &*cache_entry;
        let lod_result = lod::calc_lod(
            &cache_entry.particle_list_of_leafs,
            &cache_entry.particle_list_of_leafs_scan,
            &cache_entry.splines,
            &cache_entry.densities,
            &cache_entry.coordinates,
            cache_entry.octree.clone(),
            client_state.batch_size_lod,
            &client_state.camera_info.clone(),
            &mut client_state.level_of_detail,
        );
        Ok(web::Json(lod_result))
    } else {
        Err(ErrorInternalServerError("Something failed."))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let cfg: WebServiceConfig =
        confy::load_path("cfg.yml").expect("Failed to load config from disk");

    let cache = data_cache::DataCache::new(cfg.basedir).start();

    log::info!("starting HTTP server at http://localhost:8000");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .route("/rand", web::get().to(get_rand_init))
            .route(
                "/v1/get/splines/{simulation}/{snapshot_id}",
                web::post().to(get_snapshot),
            )
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
