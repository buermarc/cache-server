use std::time::Duration;
use actix::*;
use actix_web::{
    middleware::Logger, web, App, HttpServer, Error, rt::time::{sleep_until, Instant}
};
use actix_web::error::ErrorInternalServerError;

use cxx::CxxString;

mod data_cache;
mod load_numpy;

#[cxx::bridge]
mod ffi {

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
    fn new(x: f64, y: f64, z: f64) -> Self {
        ffi::RustVec3{ x, y, z }
    }
}


async fn get_rand_init(
    cache: web::Data<Addr<data_cache::DataCache>>,
) -> Result<String, Error> {
    sleep_until(Instant::now() + Duration::from_secs(0)).await;
    if let Ok(number) = cache.send(data_cache::RandU{}).await {
        Ok(number.to_string())
    } else {
        Err(ErrorInternalServerError("bad"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let cache = data_cache::DataCache::new().start();
    let file_name = "cpp/octree.json";

    log::info!("Loading octree");

    let octree = ffi::load_octree_from_file(file_name.to_string());
    let viewbox = ffi::Viewbox { box_min: ffi::RustVec3::new(2001.0, 2000.0, 2000.0), box_max: ffi::RustVec3::new(2504.0, 2500.0, 2506.0) };
    let values = ffi::get_intersecting_node(octree, viewbox);
    for value in values {
        println!("{}", value);
    }

    log::info!("Finished loading octree");

    log::info!("starting HTTP server at http://localhost:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(cache.clone()))
            .route("/rand", web::get().to(get_rand_init))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
