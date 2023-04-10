fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/rust_octree_bind.cpp")
        .file("src/Octree.cpp")
        .flag("-ljsoncpp")
        .compile("betterbackend");

    println!("cargo:rustc-link-lib=jsoncpp");
}