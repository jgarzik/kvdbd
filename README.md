# kvapp
Simple database service for NoSQL key/value database

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

There is no configuration file.  Limited options are available at
the command line.  Run `--help` to view available options:

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

