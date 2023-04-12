use actix::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use cxx::SharedPtr;

use ndarray::{Array1, Array2, Array3};
use ndarray_npy::read_npy;

use super::bind::ffi::{load_octree_from_file, Octree};

#[derive(Message)]
#[rtype(result = "isize")]
pub struct RandU;

#[derive(Message)]
#[rtype(result = "Arc<CacheEntry>")]
pub struct CacheRequest {
    pub simulation: String,
    pub snapshot_id: usize,
}

impl CacheRequest {
    fn as_id(&self) -> String {
        self.simulation.clone() + &self.snapshot_id.to_string()
    }
}

pub struct CacheEntry {
    pub particle_list_of_leafs: Array1<i64>,
    pub particle_list_of_leafs_scan: Array1<i64>,
    pub splines: Array3<f64>,
    pub densities: Array2<f64>,
    pub coordinates: Array2<f64>,
    pub octree: SharedPtr<Octree>,
}

pub struct DataCache {
    pub rand: isize,
    pub cache: HashMap<String, Arc<CacheEntry>>,
    pub basedir: String,
}

impl DataCache {
    pub fn new(basedir: String) -> Self {
        DataCache {
            rand: random(),
            cache: HashMap::new(),
            basedir,
        }
    }

    pub fn load_entry(&mut self, request: CacheRequest) -> Arc<CacheEntry> {
        let basedir = self.basedir.clone()
            + "/"
            + &request.simulation
            + "/"
            + &format!("snapdir_{:03}", request.snapshot_id);

        let particle_list_of_leafs = read_npy(basedir.clone() + "particle_list_of_leafs.npy")
            .expect("Failed to open {}particle_list_of_leafs");
        let particle_list_of_leafs_scan =
            read_npy(basedir.clone() + "particle_list_of_leafs_scan.npy")
                .expect("Failed to open particle_list_of_leafs_scan");
        let splines = read_npy(basedir.clone() + "splines.npy").expect("Failed to open splines");
        let densities = read_npy(basedir.clone() + "Density.npy").expect("Failed to open Density");
        let coordinates =
            read_npy(basedir.clone() + "Coordinates.npy").expect("Failed to open Coordinates");

        let octree = load_octree_from_file(basedir.clone() + "o3dOctree.json");

        let entry = Arc::new(CacheEntry {
            particle_list_of_leafs,
            particle_list_of_leafs_scan,
            splines,
            densities,
            coordinates,
            octree,
        });

        self.cache.insert(request.as_id(), entry.clone());
        entry
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

impl Handler<CacheRequest> for DataCache {
    type Result = Arc<CacheEntry>;

    fn handle(&mut self, msg: CacheRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match self.cache.get(&msg.as_id()) {
            Some(entry) => entry.clone(),
            _ => self.load_entry(msg),
        }
    }
}
