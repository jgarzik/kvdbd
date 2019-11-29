FROM rust:1.38.0 as builder
ENV NAME=kvdbd

# First build a dummy project with our dependencies to cache them in Docker
# WORKDIR /usr/src
# RUN cargo new --bin ${NAME}
# WORKDIR /usr/src/${NAME}
# COPY ./Cargo.lock ./Cargo.lock
# COPY ./Cargo.toml ./Cargo.toml
# RUN cargo build --release
# RUN rm src/*.rs
ENV PROTOBUF_VERSION=3.3.0
ENV PROTOC_FILENAME=protoc-${PROTOBUF_VERSION}-linux-x86_64.zip
RUN wget https://github.com/google/protobuf/releases/download/v${PROTOBUF_VERSION}/${PROTOC_FILENAME}
RUN unzip ${PROTOC_FILENAME}
RUN bin/protoc --version
ENV PATH=${PATH}:`pwd`/bin

# Now copy the sources and do the real build
WORKDIR /usr/src/${NAME}
COPY . .
RUN cargo test -j9
RUN cargo build --release -j9

# Second stage putting the build result into a debian stable-slim image
FROM debian:stable-slim
ENV NAME=kvdbd

COPY --from=builder /usr/src/${NAME}/target/release/${NAME} /usr/local/bin/${NAME}
EXPOSE 8080
CMD ${NAME}
