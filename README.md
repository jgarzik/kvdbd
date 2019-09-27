# kvdb

API service enabling easy remote export of embedded key/value databases.

## Table of Contents

* [Motivation](#motivation)
* [Requirements](#requirements)
* [Documentation](#documentation)
* [Using kvdb](#using-kvdb)
* [Testing](#testing)

## Motivation

A REST/JSON API web service, that enables querying key/value databases
over a network.  In effect, creating a database server for databases
that have no server.

Turn legacy, flat-file databases into micro-services.

## Requirements

* Rust 2018+
* `protoc` protobufs compiler

## Documentation

* Quick Start: see below
* Configuration:  [config.md](doc/config.md)
* Remote HTTP API:  [api.md](doc/api.md)

## Using kvdb

Standard rust cargo multi-binary setup:

```
$ cargo build
$ cargo run --bin kvdb
```

## Testing

Integration testing is performed via a separate binary, `tester`.
```
$ cargo run --bin tester
```

