use actix::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use cxx::SharedPtr;

use ndarray::{Array1, Array2, Array3};
use ndarray_npy::read_npy;

use super::bind::ffi::{load_octree_from_file, Octree};

use anyhow::Context;
use serde::Serialize;

#[derive(Message)]
#[rtype(result = "isize")]
pub struct RandU;

#[derive(Message)]
#[rtype(result = "String")]
pub struct BaseDirRequest;

#[derive(Message)]
#[rtype(result = "Arc<anyhow::Result<HashMap<String, Vec<usize>>>>")]
pub struct CachedEntriesRequest;

#[derive(Message, Eq, Hash, PartialEq, Serialize, Clone)]
#[rtype(result = "anyhow::Result<Arc<CacheEntry>>")]
pub struct CacheRequest {
    pub simulation: String,
    pub snapshot_id: usize,
}

pub struct CacheEntry {
    pub particle_list_of_leafs: Array1<i64>,
    pub particle_list_of_leafs_scan: Array1<i64>,
    pub splines: Array3<f64>,
    pub densities: Array2<f64>,
    pub quantiles: Array1<f64>,
    pub coordinates: Array2<f64>,
    pub voronoi_diameter_extended: Array1<f64>,
    pub octree: SharedPtr<Octree>,
}

pub struct DataCache {
    pub rand: isize,
    pub cache: HashMap<CacheRequest, Arc<CacheEntry>>,
    pub basedir: String,
    pub metadata_url: String,
    pub hostname: String,
}

impl DataCache {
    pub fn new(basedir: String, metadata_url: String, hostname: String) -> Self {
        DataCache {
            rand: random(),
            cache: HashMap::new(),
            basedir,
            metadata_url,
            hostname,
        }
    }

    pub fn send_info_about_cache_loading(
        &self,
        request: &CacheRequest,
    ) -> Result<ureq::Response, ureq::Error> {
        Ok(ureq::post(&(self.metadata_url.clone() + "/add_snap"))
            .set("User-Agent", &self.hostname)
            .send_json(request)?)
    }

    pub fn send_info_about_cache_loading_fail(
        &self,
        request: &CacheRequest,
    ) -> Result<ureq::Response, ureq::Error> {
        Ok(ureq::post(&(self.metadata_url.clone() + "/del_snap"))
            .set("User-Agent", &self.hostname)
            .send_json(request)?)
    }

    pub fn load_entry(&mut self, request: &CacheRequest) -> anyhow::Result<Arc<CacheEntry>> {
        /*
        log::info!("Starting async thing.");
        let other: &Self = self;
        let async_request = request.clone();
        actix_web::rt::spawn(async move {
            log::info!("Within async");
            other.send_info_about_cache_loading(&async_request)
                .await
                .inspect_err(|err| {
                    log::warn!(
                        "failed to send info about loading cache to metadata server: {:?}",
                        err
                    )
                })
                .unwrap();
        });
        */
        let basedir = self.basedir.clone()
            + "/"
            + &request.simulation
            + "/"
            + &format!("snapdir_{:03}", request.snapshot_id)
            + "/";

        let particle_list_of_leafs =
            read_npy(basedir.clone() + "particle_list_of_leafs_Density.npy")
                .context("Failed to open particle_list_of_leafs")?;
        let particle_list_of_leafs_scan =
            read_npy(basedir.clone() + "particle_list_of_leafs_Density_scan.npy")
                .context("Failed to open particle_list_of_leafs_scan")?;
        let splines =
            read_npy(basedir.clone() + "splines.npy").context("Failed to open splines")?;
        let densities: Array2<f64> =
            read_npy(basedir.clone() + "Density.npy").context("Failed to open Density")?;
        let quantiles: Array1<f64> = read_npy(basedir.clone() + "densities_quantiles.npy")
            .context("Failed to open density_quantiles")?;
        let coordinates =
            read_npy(basedir.clone() + "Coordinates.npy").context("Failed to open Coordinates")?;
        let voronoi_diameter_extended = read_npy(basedir.clone() + "voronoi_diameter_extended.npy")
            .context("Failed to open voronoi_diameter_extended")?;

        let octree = load_octree_from_file(basedir.clone() + "o3dOctree.json");

        let entry = Arc::new(CacheEntry {
            particle_list_of_leafs,
            particle_list_of_leafs_scan,
            splines,
            densities,
            quantiles,
            coordinates,
            voronoi_diameter_extended,
            octree,
        });

        self.cache.insert(request.clone(), entry.clone());
        Ok(entry)
    }

    pub fn cached_entries(&self) -> anyhow::Result<HashMap<String, Vec<usize>>> {
        let mut cached_entries = HashMap::new();
        for key in self.cache.keys() {
            let simulation = &key.simulation;
            let snapshot_id = &key.snapshot_id;
            if !cached_entries.contains_key(simulation) {
                cached_entries.insert(simulation.clone(), vec![]);
            }

            let snapshot_ids = cached_entries
                .get_mut(simulation)
                .context("Hashmap should contain vec for simulation as we just inserted it")?;
            snapshot_ids.push(*snapshot_id);
        }
        Ok(cached_entries)
    }
}

impl Actor for DataCache {
    type Context = actix::Context<Self>;
}

impl Handler<RandU> for DataCache {
    type Result = isize;

    fn handle(&mut self, _msg: RandU, _ctx: &mut actix::Context<Self>) -> Self::Result {
        self.rand
    }
}

impl Handler<BaseDirRequest> for DataCache {
    type Result = String;

    fn handle(&mut self, _msg: BaseDirRequest, _ctx: &mut actix::Context<Self>) -> Self::Result {
        self.basedir.clone()
    }
}

impl Handler<CacheRequest> for DataCache {
    type Result = anyhow::Result<Arc<CacheEntry>>;

    fn handle(&mut self, msg: CacheRequest, _ctx: &mut actix::Context<Self>) -> Self::Result {
        match self.cache.get(&msg) {
            Some(entry) => Ok(entry.clone()),
            _ => {
                self.send_info_about_cache_loading(&msg)
                    .inspect_err(|err| {
                        log::warn!(
                            "failed to send info about loading cache to metadata server: {:?}",
                            err
                        )
                    })
                    .unwrap();
                match self.load_entry(&msg) {
                    Ok(result) => return Ok(result),
                    Err(err) => {
                        log::warn!("failed to calculate load_entry {:?}", err);
                        self.send_info_about_cache_loading_fail(&msg).inspect_err(|err| log::warn!("failed to send info about loading cache to metadata server: {:?}", err)).unwrap();
                        Err(err)
                    }
                }
            }
        }
    }
}

impl Handler<CachedEntriesRequest> for DataCache {
    type Result = Arc<anyhow::Result<HashMap<String, Vec<usize>>>>;

    fn handle(
        &mut self,
        _msg: CachedEntriesRequest,
        _ctx: &mut actix::Context<Self>,
    ) -> Self::Result {
        let hashmap: anyhow::Result<HashMap<String, Vec<usize>>> = self.cached_entries();
        return Arc::new(hashmap);
    }
}
