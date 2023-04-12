use ndarray::{s, Array1, Array2, Array3};
use std::cmp::min;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::bind::ffi::{get_intersecting_node, Octree, RustVec3, Viewbox};
use cxx::SharedPtr;

#[derive(Deserialize, Clone)]
pub struct CameraInfo {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub size: f64,
}

impl CameraInfo {
    fn to_viewbox(&self) -> Viewbox {
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
    splines_a: Vec<Vec<f64>>,
    splines_b: Vec<Vec<f64>>,
    splines_c: Vec<Vec<f64>>,
    splines_d: Vec<Vec<f64>>,
    relevant_densities_flat: Vec<f64>,
    relevant_coordinates: Vec<Vec<f64>>,
    client_level_of_detail: HashMap<i64, i64>,
    min_d: f64,
    max_d: f64,
    n_particles: usize,
}

pub fn calc_lod(
    particle_list_of_leafs: &Array1<i64>,
    particle_list_of_leafs_scan: &Array1<i64>,
    splines: &Array3<f64>,
    densities: &Array2<f64>,
    coordinates: &Array2<f64>,
    octree: SharedPtr<Octree>,
    lod_batch: i64,
    camera_info: &CameraInfo,
    client_level_of_detail: &mut HashMap<i64, i64>,
) -> LodResult {
    let node_indicies = get_intersecting_node(octree, camera_info.to_viewbox());

    // length of particles in leaf can be determined using the scan
    // data = [1,2,3, 4,5,6,8, 9,10,11]
    // scan = [0,     4,       8,     ]
    // if i != scan.len() - 1
    //     len = scan[i+1] - scan[i]
    // else
    //     len = scan.len() - scan[i]

    // Go over the new idx
    // if not contained within client add them with a zero
    for idx in &node_indicies {
        if !client_level_of_detail.contains_key(idx) {
            client_level_of_detail.insert(*idx, 0);
        }
    }

    let mut relevant_ids: Vec<i64> = vec![];

    // Extract relevant particles
    for t in &node_indicies {
        let i = (*t) as usize;
        let lod = *client_level_of_detail.get(t).unwrap();

        let len = if i != particle_list_of_leafs_scan.len() - 1 {
            particle_list_of_leafs_scan[i + 1] - particle_list_of_leafs_scan[i]
        } else {
            particle_list_of_leafs.len() as i64 - particle_list_of_leafs_scan[i]
        };
        let start = particle_list_of_leafs_scan[i];
        let stop = start + len;

        let lod_start = min(start + lod * lod_batch, stop) as usize;
        let lod_end = min(start + (lod + 1) * lod_batch, stop) as usize;

        let particles = particle_list_of_leafs
            .slice(s![lod_start..lod_end])
            .to_vec();
        relevant_ids.extend(particles);
    }

    // Increase relevant LODs
    for t in &node_indicies {
        *client_level_of_detail.get_mut(t).unwrap() += 1;
    }

    let n_particles = relevant_ids.len();

    // Allocate result arrays
    // TODO unsure what is better, zero initialized and [] or with_capacity and push
    let mut splines_a: Vec<Vec<f64>> = vec![vec![]; n_particles];
    let mut splines_b: Vec<Vec<f64>> = vec![vec![]; n_particles];
    let mut splines_c: Vec<Vec<f64>> = vec![vec![]; n_particles];
    let mut splines_d: Vec<Vec<f64>> = vec![vec![]; n_particles];

    // Ugly with two vectors, but do not know a real better way
    let mut relevant_densities_flat: Vec<f64> = vec![0.0; n_particles * 2];

    // let mut relevant_coordinates: Vec<Vec<f64>> = repeat_with(|| Vec::with_capacity(3)).take(n_particles).collect();
    let mut relevant_coordinates: Vec<Vec<f64>> = vec![vec![]; n_particles];

    // Extract relevant data and copy into result arrays
    for (idx, id) in relevant_ids.into_iter().enumerate() {
        splines_a[idx] = splines.slice(s![id as usize, 0, ..]).to_vec();
        splines_b[idx] = splines.slice(s![id as usize, 1, ..]).to_vec();
        splines_c[idx] = splines.slice(s![id as usize, 2, ..]).to_vec();
        splines_d[idx] = splines.slice(s![id as usize, 3, ..]).to_vec();

        relevant_densities_flat[idx] = densities[[0, id as usize]];
        relevant_densities_flat[idx + n_particles] = densities[[1, id as usize]];

        relevant_coordinates[idx] = coordinates.slice(s![id as usize, ..]).to_vec();
    }

    let (min_d, max_d) = if n_particles > 0 {
        (
            *relevant_densities_flat
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
            *relevant_densities_flat
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        )
    } else {
        (0.0, 0.0)
    };

    LodResult {
        splines_a,
        splines_b,
        splines_c,
        splines_d,
        relevant_densities_flat,
        relevant_coordinates,
        client_level_of_detail: client_level_of_detail.to_owned(),
        min_d,
        max_d,
        n_particles,
    }
}

#[cfg(test)]
mod tests {
    use super::super::bind::ffi::load_octree_from_file;
    use super::*;
    use ndarray::{array, Array, Array1, Array2, Array3};
    use ndarray_npy::read_npy;

    #[test]
    fn test_load_numpy() {
        let basedir = env!("CARGO_MANIFEST_DIR").to_string();
        let resource = "/resource/test_data.npy";

        let data: Array1<f64> = read_npy(basedir + resource).unwrap();
        assert_eq!(data.len(), 10);
    }

    // Test will currently fail because we do not know what the octree traversal returns
    #[test]
    fn test_calc_lod_stuff() {
        let particle_list_of_leafs =
            array![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21];

        let particle_list_of_leafs_scan = array![0, 4, 7, 11, 17];

        let splines: Array3<f64> =
            Array::from_shape_fn((particle_list_of_leafs.len(), 3, 4), |(_i, j, _k)| {
                (j + 1) as f64
            });
        let densities: Array2<f64> =
            Array::from_shape_fn((2, particle_list_of_leafs.len()), |(_i, j)| (j + 1) as f64);
        let coordinates: Array2<f64> =
            Array::from_shape_fn((particle_list_of_leafs.len(), 3), |(_i, j)| (j + 1) as f64);

        let lod_batch = 2;
        let mut client_level_of_detail = HashMap::new();
        client_level_of_detail.insert(0, 1);
        client_level_of_detail.insert(1, 1);
        client_level_of_detail.insert(3, 1);

        // What is the expected return value?
        // client: [0, 1, 3];
        // new:    [1, 2, 3, 4];
        // res:    [0, 1, 2, 3, 4];
        let basedir = env!("CARGO_MANIFEST_DIR").to_string();
        let file_name = "/resource/octree.json";

        let octree = load_octree_from_file(basedir + file_name);

        let camera_info = CameraInfo {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            size: 4.0,
        };
        let res = calc_lod(
            &particle_list_of_leafs,
            &particle_list_of_leafs_scan,
            &splines,
            &densities,
            &coordinates,
            octree.clone(),
            lod_batch,
            &camera_info,
            &mut client_level_of_detail,
        );

        let mut keys: Vec<i64> = res.client_level_of_detail.keys().copied().collect();
        keys.sort();

        assert_eq!(vec![0, 1, 2, 3, 4], keys);

        assert_eq!(1, *res.client_level_of_detail.get(&0).unwrap());
        assert_eq!(2, *res.client_level_of_detail.get(&1).unwrap());
        assert_eq!(1, *res.client_level_of_detail.get(&2).unwrap());
        assert_eq!(2, *res.client_level_of_detail.get(&3).unwrap());
        assert_eq!(1, *res.client_level_of_detail.get(&4).unwrap());

        assert_eq!(7, res.n_particles)
    }
}
