# kvdbd

API service enabling easy remote export of embedded key/value databases.

Turn legacy, flat-file databases into micro-services.

## Table of Contents

* [Motivation](#motivation)
* [Requirements](#requirements)
* [Documentation](#documentation)
* [Using kvdbd](#using-kvdbd)
* [Testing](#testing)

## Motivation

A REST/JSON API web service, that enables querying key/value databases
over a network.  In effect, creating a database server for databases
that have no server.

## Requirements

* Rust 2018+
* `protoc` protobufs compiler

## Documentation

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

* Another "kvdb" exists as a Go library: https://github.com/portworx/kvdb
* An online cloud service https://kvdb.io/

