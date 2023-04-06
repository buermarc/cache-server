use npy::NpyData;
use std::io::Read;
use std::io::Error;
use ndarray::{Array1, Array2, Slice, SliceInfo, Axis};
use ndarray_npy::read_npy;
use std::collections::HashMap;
use std::cmp::min;
use std::iter::repeat_with;

pub struct CameraInfo
{
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub size: f64,
}


pub fn load_numpy<'a>(buf: &'a mut Vec<u8>, path_str: String) -> Result<NpyData<'a, f64>, Error> {
    std::fs::File::open(path_str)?.read_to_end(buf)?;
    let data: NpyData<f64> = NpyData::from_bytes(buf)?;
    return Ok(data);
}

pub fn get_node_indicies(camera_info: CameraInfo) -> Vec<i64> {
    return vec![1, 2, 3, 5];
}

pub fn calc_lod_stuff(
    particle_list_of_leafs: Array1<i64>,
    particle_list_of_leafs_scan: Array1<i64>,
    client_node_indicies: Array1<i64>,
    splines: Array2<f64>,
    densities: Array2<f64>,
    coordinates: Array2<f64>,
    lod_batch: i64,
    client_level_of_detail: HashMap<i64, i64>,
    camera_info: CameraInfo,
) 
{
    let node_indicies = get_node_indicies(camera_info).to_vec();
    let mut level_of_detail: HashMap<i64, i64> = HashMap::new();

    for idx in &node_indicies {
        level_of_detail.insert(*idx, 0);
    }

    // length of particles in leaf can be determined using the scan
    // data = [1,2,3, 4,5,6,8, 9,10,11]
    // scan = [0,     4,       8,     ]
    // if i != scan.len() - 1
    //     len = scan[i+1] - scan[i]
    // else 
    //     len = scan.len() - scan[i]

    // check which client_node_indicies are in the new node_indicies 
    let mut node_indicies_vec: Vec<i64> = node_indicies.to_vec();
    node_indicies_vec.sort();
    let node_indices_in_old_and_current_state = client_node_indicies.map(
        |x| -> bool {
            return match node_indicies_vec.binary_search(&x) { Ok(_) => true, _ => false}
        }
    );

    let mut relevant_ids: Vec<i64> = vec![];

    // If the client node are contained within the new ones, continue with their level of detail
    for (i, contained) in node_indices_in_old_and_current_state.iter().enumerate() {
        let lod = *client_level_of_detail.get(&client_node_indicies[i]).unwrap();
        if *contained {
            level_of_detail.insert(
                i as i64,
                lod
           );
        }
    }
    // TODO what do we want to do here ???
    for t in &node_indicies_vec {
        let i = *t as usize;

        let len = if i != particle_list_of_leafs_scan.len() - 1 {
            particle_list_of_leafs_scan[i + 1] - particle_list_of_leafs_scan[i]
        } else {
            particle_list_of_leafs.len() as i64 - particle_list_of_leafs_scan[i]
        };
        let start = particle_list_of_leafs_scan[i];
        let stop = start + len;

        let lod_start = min(start + lod * lod_batch, stop) as usize;
        let lod_end = min(start + (lod + 1) * lod_batch, stop) as usize;

        let as_vec = particle_list_of_leafs.to_vec();
        let particles = &as_vec[lod_start..lod_end];
        relevant_ids.extend(particles);
    }

    let n_particles = relevant_ids.len();

    let mut splines_a: Vec<f64> = Vec::with_capacity(n_particles);
    let mut splines_b: Vec<f64> = Vec::with_capacity(n_particles);
    let mut splines_c: Vec<f64> = Vec::with_capacity(n_particles);
    let mut splines_d: Vec<f64> = Vec::with_capacity(n_particles);

    let mut relevant_densities_flat: Vec<f64> = Vec::with_capacity(n_particles * 2);

    let mut relevant_coordinates: Vec<Vec<f64>> =  repeat_with(|| Vec::with_capacity(3)).take(n_particles).collect();

    let min_d = densities.iter().min_by(|a, b| a.partial_cmp(b).unwrap());
    let max_d = densities.iter().max_by(|a, b| a.partial_cmp(b).unwrap());

    for (idx, id) in relevant_ids.into_iter().enumerate() {
        splines_a[idx] = splines[[id as usize, 0]];
        splines_b[idx] = splines[[id as usize, 1]];
        splines_c[idx] = splines[[id as usize, 2]];
        splines_d[idx] = splines[[id as usize, 3]];

        relevant_densities_flat[idx] = densities[[id as usize, 0]];
        relevant_densities_flat[idx + n_particles] = densities[[id as usize, 1]];

        relevant_coordinates[idx][0] = coordinates[[id as usize, 0]];
        relevant_coordinates[idx][1] = coordinates[[id as usize, 1]];
        relevant_coordinates[idx][2] = coordinates[[id as usize, 2]];
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_numpy() {
        let basedir = env!("CARGO_MANIFEST_DIR").to_string();
        let resource = "/resource/test_data.npy";

        let data: Array1<f64> = read_npy(basedir + resource).unwrap();
        assert_eq!(data.len(), 10);
    }
}

