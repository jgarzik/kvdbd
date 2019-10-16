# kvdbd

Daemon that enables reading/writing of flat-file key/value databases
available via HTTP API, using REST/JSON or Protobufs.

## Goals

* "NoSQL databases go from filesystem to docker microservice"
* Written in safe rust
* Can be queried by stock HTTP clients
* Modern HTTP service with threads, HTTP/2 etc.
* Beyond Get/Put/Delete, expose db-specific operations such as transactions or batch-update.
* Access multiple databases simultaneously from a single HTTP endpoint.
* [Support multiple database back-ends](https://github.com/jgarzik/kvdbd/issues/6) (sled, leveldb, lmdb, gdbm, ...) (TODO)
* Docker-friendly Zero configuration mode

## Documentation

* Requirements: Rust 2018+ and `protoc` compiler.
* Quick Start: see below
* Configuration:  [config.md](doc/config.md)
* Remote HTTP API:  [api.md](doc/api.md)

## Using kvdbd

### From cargo

Standard rust cargo multi-binary setup:

```
$ cargo build
$ cargo run --bin kvdbd
```

### From docker

Zeroconf docker example, with sled database stored on docker volume `dbdata`:
```
$ docker volume create dbdata
$ docker run --rm -p 8080:8080 -v dbdata:/data jgarzik/kvdbd \
	kvdbd --bind-addr 0.0.0.0 --sled /data/sled.db
$ curl http://127.0.0.1:8080/ | json_pp
```

## Testing

Integration testing is performed via a separate binary, `tester`.
```
$ cargo run --bin tester
```

## Other projects

Unrelated projects with similar names:

* Another "kvdb" exists as a Go library: https://github.com/portworx/kvdb
* An online cloud service https://kvdb.io/

