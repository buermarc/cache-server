fn main() {
    cxx_build::bridge("src/bind.rs")
        .file("src/rust_octree_bind.cpp")
        .file("src/Octree.cpp")
        .flag("-ljsoncpp")
        .compile("cache_server");

    println!("cargo:rustc-link-lib=jsoncpp");
}
