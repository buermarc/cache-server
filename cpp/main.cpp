#include "Octree.h"
#include <iostream>

using namespace open3d;

int main() {
    const std::string path = "octree.json";
    open3d::geometry::Octree octree;
    bool result = open3d::geometry::ReadIJsonConvertibleFromJSON(path, octree);
    std::cout << result << std::endl;
    std::cout << octree.origin_ << std::endl;
    std::cout << octree.size_ << std::endl;
    std::cout << octree.max_depth_ << std::endl;
}
