use actix::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use cxx::SharedPtr;

use ndarray::{Array1, Array2};
use ndarray_npy::read_npy;

use super::bind::ffi::{Octree, load_octree_from_file};

#[derive(Message)]
#[rtype(result = "isize")]
pub struct RandU;

#[derive(Message)]
#[rtype(result = "Arc<CacheEntry>")]
pub struct CacheRequest {
    simulation: String,
    snapshot_id: usize,
}

impl CacheRequest {
    fn as_id(&self) -> String {
        self.simulation.clone() + &self.snapshot_id.to_string()
    }
}

pub struct CacheEntry {
    pub particle_list_of_leafs: Array1<i64>,
    pub particle_list_of_leafs_scan: Array1<i64>,
    pub splines: Array2<i64>,
    pub densities: Array2<i64>,
    pub coordinates: Array2<i64>,
    pub octree: SharedPtr<Octree>,
}


pub struct DataCache {
    pub rand: isize,
    pub cache: HashMap<String, Arc<CacheEntry>>,
    pub basedir: String,
}

impl DataCache {
    pub fn new(basedir: String) -> Self {
        DataCache { rand: random(), cache: HashMap::new(), basedir}
    }

    pub fn load_entry(&mut self, request: CacheRequest) -> Arc<CacheEntry> {
        let basedir = &self.basedir;
        let particle_list_of_leafs = read_npy(basedir.clone() + "particle_list_of_leafs").unwrap();
        let particle_list_of_leafs_scan = read_npy(basedir.clone() + "particle_list_of_leafs_scan").unwrap();
        let splines = read_npy(basedir.clone() + "splines").unwrap();
        let densities = read_npy(basedir.clone() + "densities").unwrap();
        let coordinates = read_npy(basedir.clone() + "coordinates").unwrap();

        let octree = load_octree_from_file("o3dOctree.json".to_string());

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
            Some(entry) => {
                entry.clone()
            }
            _ => {
                self.load_entry(msg)
            }
        }
    }
}
