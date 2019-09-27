
# Server API

Connect to HTTP endpoint using any web client.

## Table of Contents

* [Table of Contents](#table-of-contents)
* [API: Service identity and status](#api-service-identity-and-status)
* [API: BATCH-UPDATE (atomic update of many records)](#api-batch-update-atomic-update-of-many-records)
* [API: GET (lookup value by binary key)](#api-get-lookup-value-by-binary-key)
* [API: GET (lookup value by key)](#api-get-lookup-value-by-key)
* [API: PUT (store key and value)](#api-put-store-key-and-value)
* [API: PUT (store binary key and value)](#api-put-store-binary-key-and-value)
* [API: DELETE (remove record, based on binary key)](#api-delete-remove-record-based-on-binary-key)
* [API: DELETE (remove record, based on key)](#api-delete-remove-record-based-on-key)

## API: Service identity and status

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

## API: BATCH-UPDATE (atomic update of many records)

Meta-request: POST http://$HOSTNAME:$PORT/api/$DB/batch

Encode the keys/values/ into protobuf-encoded
data structure `BatchRequest`, and POST the data to /api/$DB/batch path:
```
curl -X POST --data-binary @postdata http://localhost:8080/api/db/batch
```

Returns JSON indicating success:
```
{"result":true}
```

## API: GET (lookup value by binary key)

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

## API: GET (lookup value by key)

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

## API: PUT (store key and value)

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

## API: PUT (store binary key and value)

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

## API: DELETE (remove record, based on binary key)

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

## API: DELETE (remove record, based on key)

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

