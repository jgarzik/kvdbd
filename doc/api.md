
# Server API

Connect to HTTP endpoint using any web client.

## Table of Contents

* [HTTP REST API - overview](#http-rest-api---overview)
* [REST/JSON API](#restjson-api)
   * [API: Service identity and status](#api-service-identity-and-status)
   * [API: CLEAR - delete all records](#api-clear---delete-all-records)
   * [API: DELETE - remove record, based on key](#api-delete---remove-record-based-on-key)
   * [API: GET - lookup value by key](#api-get---lookup-value-by-key)
   * [API: KEYS.json - sequential JSON list of keys in database](#api-keysjson---sequential-json-list-of-keys-in-database)
   * [API: PUT - store key and value](#api-put---store-key-and-value)
   * [API: STAT.json - database statistics](#api-statjson---database-statistics)
* [REST/Protobufs API](#restprotobufs-api)
   * [API: MUTATE - atomic update of many records](#api-batch-update---atomic-update-of-many-records)
   * [API: DELETE - remove record, based on binary key](#api-delete---remove-record-based-on-binary-key)
   * [API: GET (lookup value by binary key)](#api-get-lookup-value-by-binary-key)
   * [API: KEYS - sequential list of keys in database](#api-keys---sequential-list-of-keys-in-database)
   * [API: PUT - store binary key and value](#api-put---store-binary-key-and-value)
   * [API: STAT - database statistics](#api-stat---database-statistics)
* [kvdb-pb: Protobuf encoding utility](#kvdb-pb-protobuf-encoding-utility)

## HTTP REST API - overview

The following are the operations supported by the HTTP REST API.

Supported base protocol features:

* HTTP 1.1
* HTTP 2.0
* REST

Two encoding methods are supported:
* JSON, with some limitations on binary keys
* Protocol buffers

## REST/JSON API

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
   "name" : "kvdbd",
   "version" : "0.2.0"
}
```

### API: CLEAR - delete all records

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/clear

POST the Clear request to /api/$DB/clear path:
```
curl -X POST http://localhost:8080/api/db/clear
```

Returns JSON indicating success:
```
{"result":true}
```

### API: DELETE - remove record, based on key

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

### API: GET - lookup value by key

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

### API: KEYS.json - sequential JSON list of keys in database

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/keys.json[?lastkey=$LAST_KEY]

Encode the last-key-from-previous-query, if any, into HTTP query string,
GET the data from /api/$DB/keys.json path:
```
curl -s http://localhost:8080/api/db1/keys.json?lastkey=age
```

Returns JSON object containing a list of keys, and a continuation indicator,
if the list was truncated.  Maximum number of items returned per query: 1,000.

### API: PUT - store key and value

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

### API: STAT.json - database statistics

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/stat.json

WARNING:  This requires a full db walk, for many databases.

```
curl -s http://localhost:8080/api/db1/stat.json
```

Returns JSON object containing a record count, and other db metadata.

## REST/Protobufs API

### API: MUTATE - atomic update of many records

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/mutate

Encode the keys/values/ into protobuf-encoded
data structure `BatchRequest`, and POST the data to /api/$DB/mutate path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/mutate
```

Returns JSON indicating success:
```
{"result":true}
```

### API: DELETE - remove record, based on binary key

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/del

Encode the key into protobuf-encoded
data structure `KeyRequest`, and POST the data to /api/$DB/del path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/del
```

Returns JSON describing value found and removed (if in db):
```
{"result":true}
```

### API: GET (lookup value by binary key)

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/get

Encode the key into protobuf-encoded
data structure `KeyRequest`, and POST the data to /api/$DB/get path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/get
```

Returns binary data (application/octet-stream) describing value found,
if present:
```
25
```

### API: KEYS - sequential list of keys in database

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/keys

Encode the last-key-from-previous-query, if any, into protobuf-encoded
data structure `KeyRequest`, and POST the data to /api/$DB/keys path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/keys
```

Returns binary data (application/octet-stream) encoding the protobuf
message `KeyResponse`, which lists the keys found.
Maximum number of items returned per query: 1,000.

### API: PUT - store binary key and value

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/put

Encode the key and value into protobuf-encoded
data structure `UpdateRequest`, and POST the data to /api/$DB/put path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/put
```

Returns JSON indicating success:
```
{"result":true}
```

### API: STAT - database statistics

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/stat

WARNING:  This requires a full db walk, for many databases.

```
curl -s http://localhost:8080/api/db1/stat
```

Returns Protobuf record containing a record count, and other db metadata.

## kvdb-pb: Protobuf encoding utility

Use this tool to encode get/put protobuf commands, for use
in conjunction with curl:

```
$ cargo run --bin kvdb-pb -- --encode --key foo --value bar put > postdata
$ curl -X POST --data-binary @postdata http://localhost:8080/api/db/put
```
