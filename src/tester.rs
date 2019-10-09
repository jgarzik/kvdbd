/*
 * tester: Integration tester for kvdbd
 *
 * To be run separately from kvdbd, assuming a clean and empty db:
 * $ cargo run --bin kvdbd
 * $ cargo run --bin tester
 */

extern crate reqwest;
mod protos;

const T_ENDPOINT: &'static str = "http://127.0.0.1:8080";
const T_BASEURI: &'static str = "/api";

use reqwest::{Client, StatusCode};

use protobuf::Message;
use protos::pbapi::{BatchRequest, KeyRequest, UpdateRequest};

fn t_get_gone(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let get_url = format!("{}get", basepath);

    // encode verification get request
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec get request; key1 should not exist, following batch
    let resp_res = client.post(&get_url).body(out_bytes).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }
}

fn t_get_ok(client: &Client, db_id: String, key: String, value: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let get_url = format!("{}get", basepath);

    // encode verification get request
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec get request; key1 should not exist, following batch
    let resp_res = client.post(&get_url).body(out_bytes.clone()).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(body) => assert_eq!(body, value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

fn t_put(client: &Client, db_id: String, key: String, value: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);

    // encode put request
    let mut out_msg = UpdateRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    out_msg.set_value(value.as_bytes().to_vec());
    out_msg.set_is_insert(true);
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec put request
    let resp_res = client.post(&put_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

fn t_del(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let del_url = format!("{}del", basepath);

    // encode del request
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

fn t_del_gone(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let del_url = format!("{}del", basepath);

    // encode del request
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);

            match resp.text() {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

fn op_batch(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let batch_url = format!("{}batch", basepath);
    let test_key = String::from("op_batch_key1");
    let test_value = format!("helloworld op_put {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value);

    let mut out_msg = BatchRequest::new();

    // op1: delete
    let mut req = UpdateRequest::new();
    req.set_key("op_batch_key1".as_bytes().to_vec());
    req.set_is_insert(false);
    out_msg.reqs.push(req);

    // op2: insert
    let mut req = UpdateRequest::new();
    req.set_key("op_batch_key2".as_bytes().to_vec());
    req.set_value("op_batch_value2".as_bytes().to_vec());
    req.set_is_insert(true);
    out_msg.reqs.push(req);

    // op3: insert
    let mut req = UpdateRequest::new();
    req.set_key("op_batch_key3".as_bytes().to_vec());
    req.set_value("op_batch_value3".as_bytes().to_vec());
    req.set_is_insert(true);
    out_msg.reqs.push(req);

    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec batch request
    let resp_res = client.post(&batch_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    t_get_gone(client, db_id.clone(), test_key.clone());
    t_get_ok(
        client,
        db_id.clone(),
        String::from("op_batch_key2"),
        String::from("op_batch_value2"),
    );
    t_get_ok(
        client,
        db_id.clone(),
        String::from("op_batch_key3"),
        String::from("op_batch_value3"),
    );

    t_del(client, db_id.clone(), String::from("op_batch_key2"));
    t_del(client, db_id, String::from("op_batch_key3"));
}

fn op_del(client: &Client, db_id: String) {
    let test_key = String::from("op_del_key1");
    let test_value = format!("helloworld op_del {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value);
    t_del(client, db_id.clone(), test_key.clone());
    t_del_gone(client, db_id, test_key);
}

fn op_get(client: &Client, db_id: String) {
    let test_key = String::from("op_key1");
    let test_value = format!("helloworld op_get {}", db_id);

    t_get_gone(client, db_id.clone(), test_key.clone());
    t_put(client, db_id.clone(), test_key.clone(), test_value.clone());
    t_get_ok(client, db_id.clone(), test_key.clone(), test_value);
    t_del(client, db_id.clone(), test_key.clone());
    t_get_gone(client, db_id, test_key);
}

fn op_put(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);
    let get_url = format!("{}get", basepath);
    let del_url = format!("{}del", basepath);
    let test_key = String::from("op_put_key");
    let test_value = format!("helloworld op_put {}", db_id);

    // encode put request
    let mut out_msg = UpdateRequest::new();
    out_msg.set_key(test_key.as_bytes().to_vec());
    out_msg.set_value(test_value.as_bytes().to_vec());
    out_msg.set_is_insert(true);
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec put request
    let resp_res = client.post(&put_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // encode verification get request
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(test_key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec get request
    let resp_res = client.post(&get_url).body(out_bytes.clone()).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // re-use same KeyRequest bytes for our delete request

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }
}

fn op_obj(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let test_key = String::from("1");
    let test_value = format!("helloworld {}", db_id);

    // Check that a record with key 1 doesn't exist.
    let url = format!("{}obj/{}", basepath, test_key);
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // verify DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // PUT a new record
    let resp_res = client.put(&url).body(test_value.clone()).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

    // Check that the record exists with the correct contents.
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // Check that the record exists with the correct contents,
    // protobuf-style.
    let mut out_msg = KeyRequest::new();
    out_msg.set_key(test_key.as_bytes().to_vec());

    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    let get_pb_url = format!("{}get", basepath);
    let resp_res = client.post(&get_pb_url).body(out_bytes).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // DELETE record
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

    // Check (again) that a record with key 1 doesn't exist.
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // verify (again) DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }
}

fn main() {
    // create http client
    let client = Client::new();

    // test, for each database
    for n in 1..3 {
        let db_id = format!("db{}", n);

        op_batch(&client, db_id.clone());
        op_del(&client, db_id.clone());
        op_get(&client, db_id.clone());
        op_obj(&client, db_id.clone());
        op_put(&client, db_id.clone());
    }
    println!("Integration testing successful.");
}
