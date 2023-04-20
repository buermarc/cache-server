#pragma once
#include <memory>

#include "cache_server/include/Octree.h"
#include "cache_server/src/bind.rs.h"
#include "rust/cxx.h"

using namespace open3d::geometry;

std::shared_ptr<Octree> load_octree_from_file(rust::String file_name);
rust::Vec<int64_t> get_intersecting_node(std::shared_ptr<Octree> octree, Viewbox viewbox);
