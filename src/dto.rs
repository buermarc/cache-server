use super::bind::ffi::{RustVec3, Viewbox};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub struct CameraInfo {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub size: f64,
}

impl CameraInfo {
    pub fn to_viewbox(&self) -> Viewbox {
        let box_min = RustVec3::new(
            self.x - self.size / 2.0,
            self.y - self.size / 2.0,
            self.z - self.size / 2.0,
        );
        let box_max = RustVec3::new(
            self.x + self.size / 2.0,
            self.y + self.size / 2.0,
            self.z + self.size / 2.0,
        );
        Viewbox { box_min, box_max }
    }
}

#[derive(Serialize)]
pub struct LodResult {
    pub splines_a: Vec<f64>,
    pub splines_b: Vec<f64>,
    pub splines_c: Vec<f64>,
    pub splines_d: Vec<f64>,
    #[serde(rename = "densities")]
    pub relevant_densities_flat: Vec<f64>,
    #[serde(rename = "coordinates")]
    pub relevant_coordinates: Vec<Vec<f64>>,
    #[serde(rename = "level_of_detail")]
    pub client_level_of_detail: HashMap<i64, i64>,
    #[serde(rename = "min_density")]
    pub min_d: f64,
    #[serde(rename = "max_density")]
    pub max_d: f64,
    #[serde(rename = "nParticles")]
    pub n_particles: usize,
    #[serde(rename = "snapnum")]
    pub snapshot_id: usize,
    pub node_indices: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct WebServiceConfig {
    pub basedir: String,
    pub metadata_url: String,
    pub port: usize,
    pub cache_server_url: String,
}

#[derive(Deserialize)]
pub struct ClientState {
    pub node_indices: Vec<i64>,
    pub level_of_detail: HashMap<i64, i64>,
    pub batch_size_lod: i64,
    pub camera_information: CameraInfo,
}

#[derive(Serialize)]
pub struct InitResponse {
    #[serde(rename = "available_snaps")]
    pub all_possible_snaps: Vec<usize>,
    #[serde(rename = "BoxSize")]
    pub box_size: usize,
    #[serde(rename = "density_quantiles")]
    pub quantiles: Vec<f64>,
    pub n_quantiles: usize,
}
