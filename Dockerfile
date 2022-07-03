FROM rust:1.61 as build

RUN apt-get update && apt-get -y install protobuf-compiler

# create a new empty shell project
RUN USER=root mkdir -p /usr/src && cd /usr/src && cargo new --bin kvdbd
WORKDIR /usr/src/kvdbd

# copy your source tree
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./kvdb-lib ./kvdb-lib
COPY ./kvdb-server ./kvdb-server
COPY ./kvdb-tools ./kvdb-tools

# build for release
RUN cargo update && cargo fetch
RUN cargo build --release
RUN ( cd kvdb-server && cargo install --path . )
RUN ( cd kvdb-tools && cargo install --path . )

# our final base
FROM rust:1.61-slim-buster

# copy the build artifact from the build stage
COPY --from=build /usr/src/kvdbd/target/release/kvdbd .
COPY --from=build /usr/src/kvdbd/target/release/kvcli .
COPY --from=build /usr/src/kvdbd/target/release/tester .

# set the startup command to run your binary
CMD ["kvdbd"]
