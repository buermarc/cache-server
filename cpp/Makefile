all: include/eigen
	g++ main.cpp Octree.cpp -o main -ljsoncpp

EIG_VER=3.4.0
include/eigen:
	wget https://gitlab.com/libeigen/eigen/-/archive/3.4.0/eigen-${EIG_VER}.tar.bz2 -O include/eigen-${EIG_VER}.tar.bz2
	cd include && tar xvf eigen-${EIG_VER}.tar.bz2 && rm eigen-${EIG_VER}.tar.bz2
	mv include/eigen-${EIG_VER} include/eigen
