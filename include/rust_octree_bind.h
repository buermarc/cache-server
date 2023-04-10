#pragma once
#include <memory>

#include "betterbackend/include/Octree.h"
#include "betterbackend/src/main.rs.h"
#include "rust/cxx.h"

using namespace open3d::geometry;

std::shared_ptr<Octree> load_octree_from_file(rust::String file_name);
rust::Vec<double> get_intersecting_node(std::shared_ptr<Octree> octree, Viewbox viewbox);
