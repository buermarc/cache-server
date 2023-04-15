use actix::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use reqwest::Client;

use cxx::SharedPtr;

use ndarray::{Array1, Array2, Array3};
use ndarray_npy::read_npy;

use super::bind::ffi::{load_octree_from_file, Octree};

use serde::Serialize;

#[derive(Message)]
#[rtype(result = "isize")]
pub struct RandU;

#[derive(Message)]
#[rtype(result = "String")]
pub struct BaseDirRequest;

#[derive(Message)]
#[rtype(result = "Arc<HashMap<String, Vec<usize>>>")]
pub struct CachedEntriesRequest;

#[derive(Message, Eq, Hash, PartialEq, Serialize)]
#[rtype(result = "Arc<CacheEntry>")]
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
    pub octree: SharedPtr<Octree>,
}

pub struct DataCache {
    pub rand: isize,
    pub cache: HashMap<CacheRequest, Arc<CacheEntry>>,
    pub basedir: String,
    pub metadata_url: String,
}

impl DataCache {
    pub fn new(basedir: String, metadata_url: String) -> Self {
        DataCache {
            rand: random(),
            cache: HashMap::new(),
            basedir,
            metadata_url,
        }
    }

    pub async fn send_info_about_cache_loading(
        &self,
        request: &CacheRequest,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let client = Client::new();
        Ok(client
            .post(self.metadata_url.clone() + "/cache_addition")
            .header("User-Agent", self.rand)
            .json(request)
            .send()
            .await?)
    }

    pub fn load_entry(&mut self, request: CacheRequest) -> Arc<CacheEntry> {
        let basedir = self.basedir.clone()
            + "/"
            + &request.simulation
            + "/"
            + &format!("snapdir_{:03}", request.snapshot_id)
            + "/";

        let particle_list_of_leafs = read_npy(basedir.clone() + "particle_list_of_leafs.npy")
            .expect("Failed to open particle_list_of_leafs");
        let particle_list_of_leafs_scan =
            read_npy(basedir.clone() + "particle_list_of_leafs_scan.npy")
                .expect("Failed to open particle_list_of_leafs_scan");
        let splines = read_npy(basedir.clone() + "splines.npy").expect("Failed to open splines");
        let densities: Array2<f64> =
            read_npy(basedir.clone() + "Density.npy").expect("Failed to open Density");
        let quantiles: Array1<f64> = read_npy(basedir.clone() + "densities_quantiles.npy")
            .expect("Failed to open density_quantiles");
        let coordinates =
            read_npy(basedir.clone() + "Coordinates.npy").expect("Failed to open Coordinates");

        let octree = load_octree_from_file(basedir.clone() + "o3dOctree.json");

        let entry = Arc::new(CacheEntry {
            particle_list_of_leafs,
            particle_list_of_leafs_scan,
            splines,
            densities,
            quantiles,
            coordinates,
            octree,
        });

        self.cache.insert(request, entry.clone());
        entry
    }

    pub fn cached_entries(&self) -> HashMap<String, Vec<usize>> {
        let mut cached_entries = HashMap::new();
        for key in self.cache.keys() {
            let simulation = &key.simulation;
            let snapshot_id = &key.snapshot_id;
            if !cached_entries.contains_key(simulation) {
                cached_entries.insert(simulation.clone(), vec![]);
            }

            let snapshot_ids = cached_entries
                .get_mut(simulation)
                .expect("Hashmap should contain vec for simulation as we just inserted it");
            snapshot_ids.push(*snapshot_id);
        }
        cached_entries
    }
}

impl Actor for DataCache {
    type Context = Context<Self>;
}

impl Handler<RandU> for DataCache {
    type Result = isize;

    fn handle(&mut self, _msg: RandU, _ctx: &mut Context<Self>) -> Self::Result {
        self.rand
    }
}

impl Handler<BaseDirRequest> for DataCache {
    type Result = String;

    fn handle(&mut self, _msg: BaseDirRequest, _ctx: &mut Context<Self>) -> Self::Result {
        self.basedir.clone()
    }
}

impl Handler<CacheRequest> for DataCache {
    type Result = Arc<CacheEntry>;

    fn handle(&mut self, msg: CacheRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match self.cache.get(&msg) {
            Some(entry) => entry.clone(),
            _ => self.load_entry(msg),
        }
    }
}

impl Handler<CachedEntriesRequest> for DataCache {
    type Result = Arc<HashMap<String, Vec<usize>>>;

    fn handle(&mut self, _msg: CachedEntriesRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let hashmap: HashMap<String, Vec<usize>> = self.cached_entries();
        return Arc::new(hashmap);
    }
}
