#!/bin/sh

openssl req -x509 \
            -sha256 -days 356 \
            -nodes \
            -newkey rsa:2048 \
            -subj "/CN=127.0.0.1/C=US/L=Atlanta" \
            -keyout rootCA.key -out rootCA.crt 

openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr -config ./csr.conf
openssl x509 -req \
    -in server.csr \
    -CA rootCA.crt -CAkey rootCA.key \
    -CAcreateserial -out server.crt \
    -days 365 \
    -sha256 -extfile ./cert.conf
