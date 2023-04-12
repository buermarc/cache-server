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
    pub splines_a: Vec<Vec<f64>>,
    pub splines_b: Vec<Vec<f64>>,
    pub splines_c: Vec<Vec<f64>>,
    pub splines_d: Vec<Vec<f64>>,
    pub relevant_densities_flat: Vec<f64>,
    pub relevant_coordinates: Vec<Vec<f64>>,
    pub client_level_of_detail: HashMap<i64, i64>,
    pub min_d: f64,
    pub max_d: f64,
    pub n_particles: usize,
}

#[derive(Serialize, Deserialize)]
pub struct WebServiceConfig {
    pub basedir: String,
}

#[derive(Deserialize)]
pub struct ClientState {
    pub level_of_detail: HashMap<i64, i64>,
    pub batch_size_lod: i64,
    pub camera_info: CameraInfo,
}

#[derive(Serialize)]
pub struct InitResponse {
    pub all_possible_snaps: Vec<usize>,
    pub box_size: usize,
    pub quantiles: Vec<f64>,
    pub n_quantiles: usize,
}
