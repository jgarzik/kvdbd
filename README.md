# kvapp
Simple database service for NoSQL key/value database

## Motivation

A simple REST/JSON API web service, that demonstrates rust, web services,
and persistent state storage in a database.

## Using kvapp

```
Standard rust cargo setup:
$ cargo build
$ cargo run
```

## Server configuration

There is no configuration file.  Limited options are available at
the command line.  Run `--help` to view available options:

```
$ cargo run -- --help
```

## Server API

Connect to HTTP endpoint using any web client.

### API: Service identity and status

```
$ curl http://localhost:8080/
```

Returns JSON describing service.

### API: GET (lookup value by key)

Append the key to the URI path following the final '/'.  In the
following example, "age" is the key and "/1/db" is the base URI:
```
curl http://localhost:8080/1/db/age
```

Returns JSON describing value found (if in db).

### API: PUT (store key and value)

Append the key and value to the URI path.  In the
following example, "age" is the key, "45" is the value,
and "/1/db" is the base URI:
```
curl http://localhost:8080/1/db/age/45
```

Returns JSON indicating success.

