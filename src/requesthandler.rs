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
use anyhow::{anyhow, Context};

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
            &client_state.camera_information.clone(),
            &mut client_state.level_of_detail,
            snapshot_id,
        );
        match lod_result {
            Ok(lod_result) => Ok(web::Json(lod_result)),
            Err(err) => Err(ErrorInternalServerError(format!(
                "Failed to calculate lod result: {:?}",
                err
            ))),
        }
    } else {
        Err(ErrorInternalServerError(
            "Communication with data cache failed.",
        ))
    }
}

pub async fn get_init(
    params: web::Path<(String, usize)>,
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<impl Responder, Error> {
    match _get_init(params, cache).await {
        Ok(result) => Ok(result),
        Err(err) => Err(ErrorInternalServerError(format!(
            "Failed to calculate lod result: {:?}",
            err
        ))),
    }
}
pub async fn _get_init(
    params: web::Path<(String, usize)>,
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> anyhow::Result<web::Json<dto::InitResponse>> {
    let (simulation, snapshot_id) = (params.0.clone(), params.1);
    let message = data_cache::CacheRequest {
        simulation: simulation.to_string(),
        snapshot_id,
    };

    // Find out which snapdirs exist for this simulation
    let base = cache
        .send(data_cache::BaseDirRequest {})
        .await
        .context("Failed to get configured basedir.")?;
    let basedir = base.clone() + "/" + &simulation + "/";
    let regex = Regex::new("snapdir_.*").context("Failed to generate regex.")?;

    // For the first assume that the groupcat exists and get the box size info
    let matching_folders = utils::search_folders_matching_regex(basedir, &regex)
        .context("Failed to query for snapdirs")?;
    if matching_folders.len() == 0 {
        Err(anyhow!("No snapdirs found."))
    } else {
        let all_possible_snaps: Vec<usize> = matching_folders
            .iter()
            .into_iter()
            .map(|folder| {
                usize::from_str(
                    &folder
                        .file_name()
                        .expect("Failed to get filename.")
                        .to_string_lossy()
                        .replace("snapdir_", ""),
                )
                .expect("Failed to convert to usize")
            })
            .collect();
        let snap_id = all_possible_snaps
            .first()
            .context("Could not get first snap_id.")?;
        let groupcat = base.clone()
            + "/"
            + &simulation
            + "/"
            + &format!("groups_{:03}", snap_id)
            + "/"
            + &format!("fof_subhalo_tab_{:03}.0.hdf5", snap_id);

        let file = File::open(groupcat).context("Failed to open groupcat")?;
        let header = file
            .group("Header")
            .context("Failed to access header group")?;
        let attribute = header
            .attr("BoxSize")
            .context("Failed to access box size attribute.")?;
        let box_size = attribute
            .read_scalar::<usize>()
            .context("Failed te read scalar.")?;

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
            Err(anyhow!("Communication with data cache failed."))
        }
    }
}

pub async fn get_current_cache(
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<impl Responder, Error> {
    let message = data_cache::CachedEntriesRequest {};
    if let Ok(json) = cache.send(message).await {
        match &*json {
            Ok(json) => Ok(web::Json(json.clone())),
            Err(err) => Err(ErrorInternalServerError(format!(
                "Failed to get the current cache: {:?}",
                err
            ))),
        }
    } else {
        Err(ErrorInternalServerError(
            "Communication with data cache failed.",
        ))
    }
}
