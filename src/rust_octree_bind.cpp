#include <memory>

#include "cache_server/include/Octree.h"
#include "cache_server/include/rust_octree_bind.h"
#include <algorithm>
#include <iostream>

#include "rust/cxx.h"

template <class DstType, class SrcType>
bool IsType(SrcType* src)
{
  return dynamic_cast<DstType*>(src) != nullptr;
}

bool _box_intersect(Eigen::Vector3d min_box, Eigen::Vector3d max_box, RustVec3 min_camera, RustVec3 max_camera) {
    auto dx = std::min(max_box[0], max_camera.x) - std::max(min_box[0], min_camera.x);
    auto dy = std::min(max_box[1], max_camera.y) - std::max(min_box[1], min_camera.y);
    auto dz = std::min(max_box[2], max_camera.z) - std::max(min_box[2], min_camera.z);

    return (dx >= 0 && dy >= 0 && dz >= 0);
}


std::shared_ptr<open3d::geometry::Octree> load_octree_from_file(rust::String file_name) {
    auto octree_ptr = std::make_shared<open3d::geometry::Octree>();
    auto cpp_file_name = std::string(file_name);
    ReadIJsonConvertibleFromJSON(cpp_file_name, *octree_ptr);
    return octree_ptr;
}

rust::Vec<int64_t> get_intersecting_node(std::shared_ptr<open3d::geometry::Octree> octree, Viewbox viewbox) {
    
    rust::Vec<int64_t> particle_arr_ids;
        
    auto traverse_lambda = [&](const std::shared_ptr<open3d::geometry::OctreeNode> &node, const std::shared_ptr<open3d::geometry::OctreeNodeInfo> &node_info) {
        if (!_box_intersect((*node_info).origin_, (*node_info).origin_.array()+(*node_info).size_, viewbox.box_min, viewbox.box_max))
            return true;

        if (IsType<open3d::geometry::OctreePointColorLeafNode>(&(*node))) {

            auto cast_node = dynamic_cast<open3d::geometry::OctreePointColorLeafNode*>(&(*node));
            particle_arr_ids.push_back((int64_t)(*cast_node).indices_[0]);
            return true;
        }

        return false;

    };
    (*octree).Traverse(traverse_lambda);
    return particle_arr_ids;
}
