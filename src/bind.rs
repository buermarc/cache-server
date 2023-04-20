#[cxx::bridge]
pub mod ffi {

    struct RustVec3 {
        x: f64,
        y: f64,
        z: f64,
    }

    struct Viewbox {
        box_min: RustVec3,
        box_max: RustVec3,
    }

    #[namespace = "open3d::geometry"]
    unsafe extern "C++" {
        include!("cache-server/include/Octree.h");

        type Octree;
    }

    unsafe extern "C++" {
        include!("cache-server/include/rust_octree_bind.h");

        fn load_octree_from_file(file_name: String) -> SharedPtr<Octree>;
        fn get_intersecting_node(octree: SharedPtr<Octree>, viewbox: Viewbox) -> Vec<i64>;
    }
}

impl ffi::RustVec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        ffi::RustVec3 { x, y, z }
    }
}

unsafe impl Send for ffi::Octree {}
unsafe impl Sync for ffi::Octree {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_octree() {
        let basedir = env!("CARGO_MANIFEST_DIR").to_string();
        let file_name = "/resource/octree.json";

        let octree = ffi::load_octree_from_file(basedir + file_name);
        let viewbox = ffi::Viewbox {
            box_min: ffi::RustVec3::new(2001.0, 2000.0, 2000.0),
            box_max: ffi::RustVec3::new(2504.0, 2500.0, 2506.0),
        };
        let values = ffi::get_intersecting_node(octree, viewbox);
        println!("Length {}", values.len());
        for value in &values {
            println!("{}", value);
        }
        assert!(values.len() > 0);
    }
}
