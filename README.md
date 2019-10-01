# kvdbd

Daemon that enables reading/writing of flat-file key/value databases
available via HTTP REST/JSON API.

## Goals

* "NoSQL databases go from filesystem to docker microservice"
* Written in safe rust
* Can be queried by stock HTTP clients
* Modern HTTP service with threads, HTTP/2 etc.
* Beyond Get/Put/Delete, expose db-specific operations such as transactions or batch-update.
* Access multiple databases simultaneously from a single HTTP endpoint.
* [Support multiple database back-ends](https://github.com/jgarzik/kvdbd/issues/6) (sled, leveldb, lmdb, gdbm, ...) (TODO)
* [Zero configuration mode](https://github.com/jgarzik/kvdbd/issues/7) (TODO)

## Documentation

* Requirements: Rust 2018+ and `protoc` compiler.
* Quick Start: see below
* Configuration:  [config.md](doc/config.md)
* Remote HTTP API:  [api.md](doc/api.md)

## Using kvdbd

Standard rust cargo multi-binary setup:

```
$ cargo build
$ cargo run --bin kvdbd
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

