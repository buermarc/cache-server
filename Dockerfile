FROM rust:bullseye

WORKDIR /work

RUN DEBIAN_FRONTEND=noninteractive
RUN apt-get update 
Run apt-get install -y build-essential libjsoncpp-dev libhdf5-dev
ADD . .
RUN make  # download eigen depenency
RUN cargo build --release

CMD cargo run --release
