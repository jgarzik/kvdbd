# kvdb

API service enabling easy remote export of embedded key/value databases.

## Table of Contents

* [Motivation](#motivation)
* [Using kvdb](#using-kvdb)
* [Server configuration](#server-configuration)
    * [Configuration file](#configuration-file)
        * [databases section](#databases-section)
    * [Command line help](#command-line-help)
* [Server API](#server-api)
    * [API: Service identity and status](#api-service-identity-and-status)
    * [API: GET (lookup value by key)](#api-get-lookup-value-by-key)
    * [API: PUT (store key and value)](#api-put-store-key-and-value)
    * [API: DELETE (remove record, based on key)](#api-delete-remove-record-based-on-key)
* [Testing](#testing)

## Motivation

A REST/JSON API web service, that enables querying key/value databases
over a network.  In effect, creating a database server for databases
that have no server.

## Requirements

* Rust 2018+
* `protoc` protobufs compiler

## Using kvdb

Standard rust cargo multi-binary setup:

```
$ cargo build
$ cargo run --bin kvdb
```

## Server configuration

A JSON configuration file is required, to specify database.  Command line 
options are also available.

### Configuration file

See `example-cfg-kvdb.json` for an example configuration file.

#### databases section

The databases section contains a list of objects, each of which
describes a database to configure and expose via this API service.

Specify a short name, local path and other db attributes to configure
each database.

* **name**:  Short URI-compatible name, exposed via API at database
  name.
* **path**:  Local filesystem path to sled db directory.
* **driver**:  Database driver used to load/store data. Only valid value: "sled"

#### Misc. features section

* **debug**:  Boolean.  true, to enable additional per-request debug output.

### Command line help

Also, limited options are available at the command line.  Run `--help`
to view available options:

```
$ cargo run --bin kvdb -- --help
```

## Server API

Connect to HTTP endpoint using any web client.

### API: Service identity and status

```
$ curl http://localhost:8080/
```

Returns JSON describing service:
```
{
   "databases" : [
      {
         "name" : "db1",
         "path" : "db1.kv",
         "driver" : "sled"
      },
      {
         "name" : "db2",
         "path" : "db2.kv",
         "driver" : "sled"
      }
   ],
   "name" : "kvdb",
   "version" : "0.2.0"
}
```

### API: GET (lookup value by binary key)

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/get

Encode the key, and hash of the key, into a protobuf-encoded
data structure, and POST the data to /api/$DB/key path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/get
```

Returns binary data (application/octet-stream) describing value found,
if present:
```
25
```

### API: GET (lookup value by key)

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/obj/$KEY

Append the key to the URI path following the final '/'.  In the
following example, "age" is the key and "/api/db" is the base URI:
```
curl http://localhost:8080/api/db/obj/age
```

Returns binary data (application/octet-stream) describing value found,
if present:
```
25
```

### API: PUT (store key and value)

Meta-request: PUT http://$HOSTNAME:$PORT/api/$DB/obj/$KEY

Append the key to the URI path, and provide HTTP body as value.  In the
following example, "age" is the key, "25" is the value,
and "/api/db" is the base URI:
```
curl --data-binary 25 -X PUT http://localhost:8080/api/db/obj/age
```

Returns JSON indicating success:
```
{"result":true}
```

### API: DELETE (remove record, based on key)

Meta-request: DELETE http://$HOSTNAME:$PORT/api/$DB/obj/$KEY

Append the key to the URI path following the final '/'.  In the
following example, "age" is the key associated with the record
being removed, and "/api/db" is the base URI:
```
curl -X DELETE http://localhost:8080/api/db/obj/age
```

Returns JSON describing value found and removed (if in db):
```
{"result":true}
```

## Testing

Integration testing is performed via a separate binary, `tester`.
```
$ cargo run --bin tester
```

