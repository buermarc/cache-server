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

    #[namespace="open3d::geometry"]
    unsafe extern "C++" {
        include!("betterbackend/include/Octree.h");

        type Octree;
    }

    unsafe extern "C++" {
        include!("betterbackend/include/rust_octree_bind.h");

        fn load_octree_from_file(file_name: String) -> SharedPtr<Octree>;
        fn get_intersecting_node(octree: SharedPtr<Octree>, viewbox: Viewbox) -> Vec<f64>;
    }
}


impl ffi::RustVec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        ffi::RustVec3{ x, y, z }
    }
}
