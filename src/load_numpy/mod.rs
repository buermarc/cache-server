use npy::NpyData;
use std::io::Read;
use std::io::Error;
use ndarray::{Array, Array1, Array2, Slice, SliceInfo, Axis, array};
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

pub struct LodResult {
    splines_a: Vec<f64>,
    splines_b: Vec<f64>,
    splines_c: Vec<f64>,
    splines_d: Vec<f64>,
    relevant_densities_flat: Vec<f64>,
    relevant_coordinates: Vec<Vec<f64>>,
    client_level_of_detail: HashMap<i64, i64>,
    min_d: f64,
    max_d: f64,
    n_particles: usize
}



pub fn load_numpy<'a>(buf: &'a mut Vec<u8>, path_str: String) -> Result<NpyData<'a, f64>, Error> {
    std::fs::File::open(path_str)?.read_to_end(buf)?;
    let data: NpyData<f64> = NpyData::from_bytes(buf)?;
    return Ok(data);
}

pub fn get_node_indicies(camera_info: CameraInfo) -> Vec<i64> {
    return vec![1, 2, 3, 4];
}

pub fn calc_lod_stuff(
    particle_list_of_leafs: Array1<i64>,
    particle_list_of_leafs_scan: Array1<i64>,
    splines: Array2<f64>,
    densities: Array2<f64>,
    coordinates: Array2<f64>,
    lod_batch: i64,
    client_level_of_detail: &mut HashMap<i64, i64>,
    camera_info: CameraInfo,
) -> LodResult
{
    let node_indicies = get_node_indicies(camera_info).to_vec();

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
    let as_vec = particle_list_of_leafs.to_vec();
    for t in &node_indicies {
        let i = *t as usize;
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

        let particles = &as_vec[lod_start..lod_end];
        relevant_ids.extend(particles);
    }

    // Increase relevant LODs
    for t in &node_indicies {
        *client_level_of_detail.get_mut(t).unwrap() += 1;
    }

    let n_particles = relevant_ids.len();

    // Allocate result arrays
    // TODO unsure what is better, zero initialized and [] or with_capacity and push
    let mut splines_a: Vec<f64> = vec![0.0; n_particles];
    let mut splines_b: Vec<f64> = vec![0.0; n_particles];
    let mut splines_c: Vec<f64> = vec![0.0; n_particles];
    let mut splines_d: Vec<f64> = vec![0.0; n_particles];

    // Ugly with two vectors, but do not know a real better way
    let mut relevant_densities_flat: Vec<f64> = vec![0.0; n_particles * 2]; 

    let mut relevant_coordinates: Vec<Vec<f64>> =  repeat_with(|| Vec::with_capacity(3)).take(n_particles).collect();

    let min_d = *densities.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let max_d = *densities.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

    // Extract relevant data and copy into result arrays
    for (idx, id) in relevant_ids.into_iter().enumerate() {
        splines_a[idx] = splines[[id as usize, 0]];
        splines_b[idx] = splines[[id as usize, 1]];
        splines_c[idx] = splines[[id as usize, 2]];
        splines_d[idx] = splines[[id as usize, 3]];

        relevant_densities_flat[idx] = densities[[id as usize, 0]];
        relevant_densities_flat[idx + n_particles] = densities[[id as usize, 1]];

        relevant_coordinates[idx].push(coordinates[[id as usize, 0]]);
        relevant_coordinates[idx].push(coordinates[[id as usize, 1]]);
        relevant_coordinates[idx].push(coordinates[[id as usize, 2]]);
    }

    return LodResult {
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
    use super::*;
    #[test]
    fn test_load_numpy() {
        let basedir = env!("CARGO_MANIFEST_DIR").to_string();
        let resource = "/resource/test_data.npy";

        let data: Array1<f64> = read_npy(basedir + resource).unwrap();
        assert_eq!(data.len(), 10);
    }

    #[test]
    fn test_calc_lod_stuff() {
        let particle_list_of_leafs = array![1,2,3,4, 5,6,7, 8,9,10,11,  12,13,14,15,16,17, 18,19,20,21];

        let particle_list_of_leafs_scan = array![0, 4, 7, 11, 17];

        let splines: Array2<f64> = Array::from_shape_fn((particle_list_of_leafs.len(), 4), |(_i, j)| (j + 1) as f64);
        let densities: Array2<f64> = Array::from_shape_fn((particle_list_of_leafs.len(), 2), |(_i, j)| (j + 1) as f64);
        let coordinates: Array2<f64> = Array::from_shape_fn((particle_list_of_leafs.len(), 3), |(_i, j)| (j + 1) as f64);

        let lod_batch = 2;
        let mut client_level_of_detail = HashMap::new();
        client_level_of_detail.insert(0, 1);
        client_level_of_detail.insert(1, 1);
        client_level_of_detail.insert(3, 1);

        // What is the expected return value?
        // client: [0, 1, 3];
        // new:    [1, 2, 3, 4];
        // res:    [0, 1, 2, 3, 4];

        let camera_info = CameraInfo { x: 1.0, y: 2.0, z: 3.0, size: 4.0 };
        let res = calc_lod_stuff(
            particle_list_of_leafs,
            particle_list_of_leafs_scan,
            splines,
            densities,
            coordinates,
            lod_batch,
            &mut client_level_of_detail,
            camera_info,
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

