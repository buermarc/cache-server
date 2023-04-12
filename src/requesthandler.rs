use actix::*;
use actix_web::error::ErrorInternalServerError;
use actix_web::{
    rt::time::{sleep_until, Instant},
    web, Error, Responder,
};
use std::time::Duration;

use std::str::FromStr;

use regex::Regex;

use hdf5::File;

use super::{data_cache, dto, lod, utils};

pub async fn get_rand_init(cache: web::Data<Addr<data_cache::DataCache>>) -> Result<String, Error> {
    sleep_until(Instant::now() + Duration::from_secs(0)).await;
    if let Ok(number) = cache.send(data_cache::RandU {}).await {
        Ok(number.to_string())
    } else {
        Err(ErrorInternalServerError("bad"))
    }
}

pub async fn get_snapshot(
    params: web::Path<(String, usize)>,
    mut client_state: web::Json<dto::ClientState>,
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

pub async fn get_init(
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
    let matching_folders = utils::search_folders_matching_regex(basedir, &regex)
        .expect("Failed to query for snapdirs");
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
            let init_response = dto::InitResponse {
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
