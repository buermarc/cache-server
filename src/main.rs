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

use std::str::FromStr;

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

use hdf5::File;

mod bind;
mod data_cache;
mod lod;

fn search_folders_matching_regex<P: AsRef<Path>>(
    path: P,
    re: &Regex,
) -> Result<Vec<PathBuf>, Error> {
    let mut matching_folders = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let folder_name = path.file_name().unwrap().to_string_lossy();

            if re.is_match(&folder_name) {
                matching_folders.push(path.clone());
            }

            // Recursively search subfolders
            let mut subfolder_matches = search_folders_matching_regex(path, re)?;
            matching_folders.append(&mut subfolder_matches);
        }
    }

    Ok(matching_folders)
}

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

#[derive(Serialize)]
pub struct InitResponse {
    pub all_possible_snaps: Vec<usize>,
    pub box_size: usize,
    pub quantiles: Vec<f64>,
    pub n_quantiles: usize,
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

async fn get_init(
    params: web::Path<(String, usize)>,
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<impl Responder, Error> {
    let (simulation, snapshot_id) = (params.0.clone(), params.1);
    let message = data_cache::CacheRequest {
        simulation: simulation.to_string(),
        snapshot_id,
    };

    // Find out which snapdirs exist for this simulation
    let base = cache
        .send(data_cache::BaseDirRequest {})
        .await
        .expect("Failed to get configured basedir.");
    let basedir = base.clone() + "/" + &simulation + "/";
    let regex = Regex::new("snapdir_.*").expect("Failed to generate regex.");
    // For the first assume that the groupcat exists and get the box size info

    let matching_folders =
        search_folders_matching_regex(basedir, &regex).expect("Failed to query for snapdirs");
    if matching_folders.len() == 0 {
        Err(ErrorInternalServerError("No snapdirs found."))
    } else {
        let all_possible_snaps: Vec<usize> = matching_folders
            .iter()
            .into_iter()
            .map(|folder| {
                usize::from_str(
                    &folder
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace("snapdir_", ""),
                )
                .expect("Failed to convert to usize")
            })
            .collect();
        let snap_id = all_possible_snaps
            .first()
            .expect("Could not get first snap_id.");
        let groupcat = base.clone()
            + "/"
            + &simulation
            + "/"
            + &format!("groups_{:03}", snap_id)
            + "/"
            + &format!("fof_subhalo_tab_{:03}.0.hdf5", snap_id);

        let file = File::open(groupcat).expect("Failed to open groupcat");
        let header = file.group("Header").expect("Failed to access header group");
        let attribute = header
            .attr("BoxSize")
            .expect("Failed to access box size attribute.");
        let box_size = attribute
            .read_scalar::<usize>()
            .expect("Failed te read scalar.");

        if let Ok(cache_entry) = cache.send(message).await {
            let cache_entry = &*cache_entry;
            let init_response = InitResponse {
                all_possible_snaps,
                box_size,
                quantiles: cache_entry.quantiles.to_vec(),
                n_quantiles: cache_entry.quantiles.len(),
            };
            Ok(web::Json(init_response))
        } else {
            Err(ErrorInternalServerError("Something failed."))
        }
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
            .route(
                "/v1/get/init/{simulation}/{snapshot_id}",
                web::get().to(get_init),
            )
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
