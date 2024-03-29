
# Server API

Connect to HTTP endpoint using any web client.

## Table of Contents

* [HTTP REST API - overview](#http-rest-api---overview)
* [REST/JSON API](#restjson-api)
   * [API: Service identity and status](#api-service-identity-and-status)
   * [API: STAT.json - database statistics](#api-statjson---database-statistics)
* [REST/Protobufs API](#restprotobufs-api)
   * [API: CLEAR - delete all records](#api-clear---delete-all-records)
   * [API: MUTATE - atomic update of many records](#api-batch-update---atomic-update-of-many-records)
   * [API: DELETE - remove record, based on binary key](#api-delete---remove-record-based-on-binary-key)
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

[Protocol Buffers](https://developers.google.com/protocol-buffers)
define the network protocol and wire encoding.

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

### API: STAT.json - database statistics

Meta-request: GET http://$HOSTNAME:$PORT/api/$DB/stat.json

WARNING:  This requires a full db walk, for many databases.

```
curl -s http://localhost:8080/api/db1/stat.json
```

Returns JSON object containing a record count, and other db metadata.

## REST/Protobufs API

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

### API: Multiple-GET (lookup multiple values by binary keys)

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/mget

Encode the key into protobuf-encoded
data structure `GetRequest`, and POST the data to /api/$DB/mget path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/mget
```

Returns binary, protobuf-encoded data structure `GetResponse`,
containing multiple results, in the order and number found in the
`GetRequest` sent.

### API: ITER - sequential list of keys in database

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/iter

Encode the last-key-from-previous-query, if any, into protobuf-encoded
data structure `KeyRequest`, and POST the data to /api/$DB/iter path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/iter
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
