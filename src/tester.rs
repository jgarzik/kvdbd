/*
 * tester: Integration tester for kvdbd
 *
 * To be run separately from kvdbd, assuming a clean and empty db:
 * $ cargo run --bin kvdbd
 * $ cargo run --bin tester
 */

extern crate clap;
extern crate reqwest;
mod protos;

const T_ENDPOINT: &'static str = "http://127.0.0.1:8080";
const T_BASEURI: &'static str = "/api";

const APPNAME: &'static str = "kvdbd-tester";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

use reqwest::{Client, StatusCode};

use protobuf::parse_from_bytes;
use protobuf::Message;
use protos::pbapi::{
    BatchRequest, BatchRequest_MagicNum, DbStatResponse, IterRequest, IterRequest_MagicNum,
    KeyRequest, KeyRequest_MagicNum, KeyResponse, UpdateRequest, UpdateRequest_MagicNum,
};

struct KeyList {
    keys: Vec<Vec<u8>>,
    list_end: bool,
}

fn t_iter(client: &Client, db_id: String, start_key: Option<Vec<u8>>) -> KeyList {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let keys_url = format!("{}keys", basepath);

    // encode keys request
    let mut out_msg = IterRequest::new();
    out_msg.magic = IterRequest_MagicNum::MAGIC;
    match start_key {
        None => out_msg.set_start_key(Vec::new()),
        Some(s) => out_msg.set_start_key(s),
    }
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec keys request; check for successful response
    let resp_res = client.post(&keys_url).body(out_bytes).send();
    if resp_res.is_err() {
        assert!(false);
    }
    let mut resp = resp_res.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // decode protobuf list-of-keys response
    let mut body: Vec<u8> = vec![];
    match resp.copy_to(&mut body) {
        Err(_e) => assert!(false),
        Ok(_o) => {}
    }
    let in_msg;
    match parse_from_bytes::<KeyResponse>(&body) {
        Err(_e) => {
            assert!(false);
            panic!("silence E0381 warning");
        }
        Ok(req) => {
            in_msg = req;
        }
    }

    // copy from pb struct to normal struct for returning data
    let mut key_list: Vec<Vec<u8>> = Vec::new();
    for key in in_msg.get_keys() {
        key_list.push(key.clone());
    }

    KeyList {
        keys: key_list,
        list_end: in_msg.list_end,
    }
}

fn t_get_gone(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let get_url = format!("{}get", basepath);

    // encode verification get request
    let mut out_msg = KeyRequest::new();
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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

fn t_put_bytes(client: &Client, db_id: String, key: &[u8], value: &[u8]) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);

    // encode put request
    let mut out_msg = UpdateRequest::new();
    out_msg.magic = UpdateRequest_MagicNum::MAGIC;
    out_msg.set_key(key.to_vec());
    out_msg.set_value(value.to_vec());
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

fn t_put(client: &Client, db_id: String, key: String, value: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);

    // encode put request
    let mut out_msg = UpdateRequest::new();
    out_msg.magic = UpdateRequest_MagicNum::MAGIC;
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
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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
    out_msg.magic = BatchRequest_MagicNum::MAGIC;

    // op1: delete
    let mut req = UpdateRequest::new();
    req.magic = UpdateRequest_MagicNum::MAGIC;
    req.set_key("op_batch_key1".as_bytes().to_vec());
    req.set_is_insert(false);
    out_msg.reqs.push(req);

    // op2: insert
    let mut req = UpdateRequest::new();
    req.magic = UpdateRequest_MagicNum::MAGIC;
    req.set_key("op_batch_key2".as_bytes().to_vec());
    req.set_value("op_batch_value2".as_bytes().to_vec());
    req.set_is_insert(true);
    out_msg.reqs.push(req);

    // op3: insert
    let mut req = UpdateRequest::new();
    req.magic = UpdateRequest_MagicNum::MAGIC;
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

fn op_stat(client: &Client, db_id: String) {
    let test_key = String::from("op_stat_key1");
    let test_value = format!("helloworld op_stat {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value);

    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let stat_url = format!("{}stat", basepath);

    // exec db-stat request
    let resp_res = client.get(&stat_url).send();
    if resp_res.is_err() {
        assert!(false);
    }
    let mut resp = resp_res.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // decode protobuf list-of-keys response
    let mut body: Vec<u8> = vec![];
    match resp.copy_to(&mut body) {
        Err(_e) => assert!(false),
        Ok(_o) => {}
    }
    let in_msg;
    match parse_from_bytes::<DbStatResponse>(&body) {
        Err(_e) => {
            assert!(false);
            panic!("silence E0381 warning");
        }
        Ok(req) => {
            in_msg = req;
        }
    }

    assert_eq!(in_msg.n_records, 1);
}

fn op_iter(client: &Client, db_id: String) {
    const DATA_COUNT: usize = 2001;
    let mut vdata: Vec<Vec<u8>> = Vec::new();

    // for simplicity, key==value in this test

    for i in 0..DATA_COUNT {
        let s = format!("datum {}", i);
        vdata.push(s.as_bytes().to_vec());
    }

    for s in &vdata {
        t_put_bytes(client, db_id.clone(), &s, &s);
    }

    let mut check_data: Vec<Vec<u8>> = Vec::new();

    let mut last_key: Option<Vec<u8>> = None;
    let mut list_end = false;
    while !list_end {
        let key_list = t_iter(client, db_id.clone(), last_key.clone());

        for key in key_list.keys {
            check_data.push(key.clone());
            last_key = Some(key.clone());
        }

        list_end = key_list.list_end;
    }

    vdata.sort();
    check_data.sort();

    for i in 0..DATA_COUNT {
        let s1 = String::from_utf8(vdata[i].clone()).unwrap();
        let s2 = String::from_utf8(check_data[i].clone()).unwrap();
        assert_eq!(s1, s2);
    }
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

fn op_clear(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let clear_url = format!("{}clear", basepath);
    let test_key = String::from("op_clear_key");
    let test_value = format!("helloworld op_clear {}", db_id);

    t_get_gone(client, db_id.clone(), test_key.clone());
    t_put(client, db_id.clone(), test_key.clone(), test_value.clone());
    t_get_ok(client, db_id.clone(), test_key.clone(), test_value);

    // exec clear-db request
    let resp_res = client.post(&clear_url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

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
    out_msg.magic = UpdateRequest_MagicNum::MAGIC;
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
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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
    out_msg.magic = KeyRequest_MagicNum::MAGIC;
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
    // CLI parser static setup
    let cli_app = clap::App::new(APPNAME)
        .version(VERSION)
        .author("Jeff Garzik <jgarzik@pobox.com>")
        .about("Integration tester for kvdbd");

    // parse command line
    let _cli_matches = cli_app.get_matches();

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
        op_clear(&client, db_id.clone());
        op_stat(&client, db_id.clone());
        op_iter(&client, db_id.clone());
    }
    println!("Integration testing successful.");
}
