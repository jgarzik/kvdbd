# kvapp
Simple database service for NoSQL key/value database

## Table of Contents

      * [Motivation](#motivation)
      * [Using kvapp](#using-kvapp)
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

A simple REST/JSON API web service, that demonstrates rust, web services,
and persistent state storage in a database.

## Using kvapp

Standard rust cargo multi-binary setup:
```
$ cargo build
$ cargo run --bin kvapp
```

## Server configuration

A JSON configuration file is required, to specify database.  Command line 
options are also available.

### Configuration file

See `example-cfg-kvapp.json` for an example configuration file.

#### databases section

The databases section contains a list of objects, each of which
describes a database to configure and expose via this API service.

Specify a short name, local path and other db attributes to configure
each database.

* **name**:  Short URI-compatible name, exposed via API at database
  name.
* **path**:  Local filesystem path to sled db directory.

### Command line help

Also, limited options are available at the command line.  Run `--help`
to view available options:

```
$ cargo run --bin kvapp -- --help
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
         "name" : "db"
      }
   ],
   "version" : "0.1.0",
   "name" : "kvapp"
}
```

### API: GET (lookup value by key)

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/$KEY

Append the key to the URI path following the final '/'.  In the
following example, "age" is the key and "/api/db" is the base URI:
```
curl http://localhost:8080/api/db/age
```

Returns binary data (application/octet-stream) describing value found,
if present:
```
25
```

### API: PUT (store key and value)

Meta-request: PUT http://$HOSTNAME:$PORT/api/$DB/$KEY

Append the key to the URI path, and provide HTTP body as value.  In the
following example, "age" is the key, "25" is the value,
and "/api/db" is the base URI:
```
curl --data-binary 25 -X PUT http://localhost:8080/api/db/age
```

Returns JSON indicating success:
```
{"result":true}
```

### API: DELETE (remove record, based on key)

Meta-request: DELETE http://$HOSTNAME:$PORT/api/$DB/$KEY

Append the key to the URI path following the final '/'.  In the
following example, "age" is the key associated with the record
being removed, and "/api/db" is the base URI:
```
curl -X DELETE http://localhost:8080/api/db/age
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

