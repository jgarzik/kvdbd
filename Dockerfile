FROM rust:1.61 as build

RUN apt-get update && apt-get -y install protobuf-compiler

# create a new empty shell project
RUN USER=root mkdir -p /usr/src && cd /usr/src && cargo new --bin kvdbd
WORKDIR /usr/src/kvdbd

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cp src/main.rs src/kvdb-pb.rs
RUN cp src/main.rs src/tester.rs

# this build step will cache your dependencies
RUN cargo update && cargo fetch
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./build.rs ./build.rs
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/kvdbd* ./target/release/deps/kvdb_pb* ./target/release/deps/tester*
RUN cargo build --release
RUN cargo install --path .

# our final base
FROM rust:1.61-slim-buster

# copy the build artifact from the build stage
COPY --from=build /usr/src/kvdbd/target/release/kvdbd .
COPY --from=build /usr/src/kvdbd/target/release/kvdb-pb .
COPY --from=build /usr/src/kvdbd/target/release/tester .

# set the startup command to run your binary
CMD ["kvdbd"]
