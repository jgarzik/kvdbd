/*
 * tester: Integration tester for kvdbd
 *
 * To be run separately from kvdbd, assuming a clean and empty db:
 * $ cargo run --bin kvdbd
 * $ cargo run --bin tester
 */

extern crate clap;
extern crate reqwest;

const T_ENDPOINT: &'static str = "https://127.0.0.1:8080";
const T_BASEURI: &'static str = "/api";

const APPNAME: &'static str = "kvdbd-tester";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

use reqwest::{Client, StatusCode};

use protobuf::{EnumOrUnknown, Message};
include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use pbapi::{
    get_op_result, get_request, get_response, iter_request, key_request, mutation_request,
    update_request, DbStatResponse, GetOp, GetRequest, GetResponse, IterRequest, KeyRequest,
    KeyResponse, MutationRequest, UpdateRequest,
};

struct KeyList {
    keys: Vec<Vec<u8>>,
    list_end: bool,
}

fn pbenc_iter_req(start_key: Option<Vec<u8>>, prefix: Option<Vec<u8>>) -> Vec<u8> {
    let mut out_msg = IterRequest::new();
    out_msg.magic = EnumOrUnknown::new(iter_request::MagicNum::MAGIC);
    match start_key {
        None => out_msg.start_key = Vec::new(),
        Some(s) => out_msg.start_key = s,
    }
    match prefix {
        None => out_msg.prefix = Vec::new(),
        Some(s) => out_msg.prefix = s,
    }
    return out_msg.write_to_bytes().unwrap();
}

fn pbenc_get1_req(key: &[u8], skip_val: bool) -> Vec<u8> {
    let mut out_msg = GetRequest::new();
    out_msg.magic = EnumOrUnknown::new(get_request::MagicNum::MAGIC);

    let mut out_op = GetOp::new();
    out_op.key = key.to_vec();
    out_op.skip_val = skip_val;
    out_msg.ops.push(out_op);

    return out_msg.write_to_bytes().unwrap();
}

fn pbenc_key_req(key: &[u8]) -> Vec<u8> {
    let mut out_msg = KeyRequest::new();
    out_msg.magic = EnumOrUnknown::new(key_request::MagicNum::MAGIC);
    out_msg.key = key.to_vec();
    return out_msg.write_to_bytes().unwrap();
}

fn pbenc_update_ins(key: &[u8], val: &[u8]) -> UpdateRequest {
    let mut out_msg = UpdateRequest::new();
    out_msg.magic = EnumOrUnknown::new(update_request::MagicNum::MAGIC);
    out_msg.key = key.to_vec();
    out_msg.value = val.to_vec();
    out_msg.is_insert = true;

    out_msg
}

fn pbenc_update_del(key: &[u8]) -> UpdateRequest {
    let mut out_msg = UpdateRequest::new();
    out_msg.magic = EnumOrUnknown::new(update_request::MagicNum::MAGIC);
    out_msg.key = key.to_vec();
    out_msg.is_insert = false;

    out_msg
}

fn pbenc_update_req(key: &[u8], val: &[u8]) -> Vec<u8> {
    let out_msg = pbenc_update_ins(key, val);
    return out_msg.write_to_bytes().unwrap();
}

async fn t_iter(client: &Client, db_id: String, start_key: Option<Vec<u8>>) -> KeyList {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let keys_url = format!("{}keys", basepath);

    // encode keys request
    let out_bytes = pbenc_iter_req(start_key, None);

    // exec keys request; check for successful response
    let resp_res = client.post(&keys_url).body(out_bytes).send().await;
    if resp_res.is_err() {
        assert!(false);
    }
    let resp = resp_res.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // decode protobuf list-of-keys response
    let bytes_res = resp.bytes().await;
    let bytes;
    match bytes_res {
        Ok(bytes_) => bytes = bytes_,
        Err(_e) => {
            assert!(false);
            panic!("silence E0381 warning");
        }
    }

    let in_msg;
    match KeyResponse::parse_from_bytes(&bytes) {
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
    for key in in_msg.keys {
        key_list.push(key.clone());
    }

    KeyList {
        keys: key_list,
        list_end: in_msg.list_end,
    }
}

async fn t_get_gone(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let get_url = format!("{}mget", basepath);

    // encode get request
    let out_bytes = pbenc_get1_req(key.as_bytes(), true);

    // exec get request; key1 should not exist, following batch
    let resp_res = client.post(&get_url).body(out_bytes.clone()).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.bytes().await {
                Ok(bytes) => match GetResponse::parse_from_bytes(&bytes) {
                    Err(_e) => assert!(false),
                    Ok(in_resp) => {
                        assert_eq!(
                            in_resp.magic,
                            EnumOrUnknown::new(get_response::MagicNum::MAGIC)
                        );
                        assert_eq!(in_resp.res.len(), 1);

                        assert_eq!(in_resp.res[0].is_ok, false);
                        assert_eq!(
                            in_resp.res[0].err,
                            EnumOrUnknown::new(get_op_result::GetErr::KEY_NOT_FOUND)
                        );
                    }
                },
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

async fn t_get_ok(client: &Client, db_id: String, key: String, value: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let get_url = format!("{}mget", basepath);

    // encode get request
    let out_bytes = pbenc_get1_req(key.as_bytes(), false);

    // exec get request; key1 should exist and match value, following batch
    let resp_res = client.post(&get_url).body(out_bytes.clone()).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.bytes().await {
                Ok(bytes) => match GetResponse::parse_from_bytes(&bytes) {
                    Err(_e) => assert!(false),
                    Ok(in_resp) => {
                        assert_eq!(
                            in_resp.magic,
                            EnumOrUnknown::new(get_response::MagicNum::MAGIC)
                        );
                        assert_eq!(in_resp.res.len(), 1);

                        assert_eq!(in_resp.res[0].is_ok, true);
                        assert_eq!(in_resp.res[0].val, value.as_bytes());
                    }
                },
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

async fn t_put_bytes(client: &Client, db_id: String, key: &[u8], value: &[u8]) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);

    // encode put request
    let out_bytes = pbenc_update_req(key, value);

    // exec put request
    let resp_res = client.post(&put_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

async fn t_put(client: &Client, db_id: String, key: String, value: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);

    // encode put request
    let out_bytes = pbenc_update_req(key.as_bytes(), value.as_bytes());

    // exec put request
    let resp_res = client.post(&put_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => {
            println!("PUT-err {}", _e);
            assert!(false)
        }
    }
}

async fn t_del(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let del_url = format!("{}del", basepath);

    // encode del request
    let out_bytes = pbenc_key_req(key.as_bytes());

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

async fn t_del_gone(client: &Client, db_id: String, key: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let del_url = format!("{}del", basepath);

    // encode del request
    let out_bytes = pbenc_key_req(key.as_bytes());

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }
}

async fn op_batch(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let mutate_url = format!("{}mutate", basepath);
    let test_key = String::from("op_batch_key1");
    let test_value = format!("helloworld op_put {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value).await;

    let mut out_msg = MutationRequest::new();
    out_msg.magic = EnumOrUnknown::new(mutation_request::MagicNum::MAGIC);

    // op1: delete
    let req = pbenc_update_del("op_batch_key1".as_bytes());
    out_msg.reqs.push(req);

    // op2: insert
    let req = pbenc_update_ins("op_batch_key2".as_bytes(), "op_batch_value2".as_bytes());
    out_msg.reqs.push(req);

    // op3: insert
    let req = pbenc_update_ins("op_batch_key3".as_bytes(), "op_batch_value3".as_bytes());
    out_msg.reqs.push(req);

    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    // exec batch request
    let resp_res = client.post(&mutate_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    t_get_gone(client, db_id.clone(), test_key.clone()).await;
    t_get_ok(
        client,
        db_id.clone(),
        String::from("op_batch_key2"),
        String::from("op_batch_value2"),
    )
    .await;
    t_get_ok(
        client,
        db_id.clone(),
        String::from("op_batch_key3"),
        String::from("op_batch_value3"),
    )
    .await;

    t_del(client, db_id.clone(), String::from("op_batch_key2")).await;
    t_del(client, db_id, String::from("op_batch_key3")).await;
}

async fn op_del(client: &Client, db_id: String) {
    let test_key = String::from("op_del_key1");
    let test_value = format!("helloworld op_del {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value).await;
    t_del(client, db_id.clone(), test_key.clone()).await;
    t_del_gone(client, db_id, test_key).await;
}

async fn op_stat(client: &Client, db_id: String) {
    let test_key = String::from("op_stat_key1");
    let test_value = format!("helloworld op_stat {}", db_id);

    t_put(client, db_id.clone(), test_key.clone(), test_value).await;

    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let stat_url = format!("{}stat", basepath);

    // exec db-stat request
    let resp_res = client.get(&stat_url).send().await;
    if resp_res.is_err() {
        assert!(false);
    }
    let resp = resp_res.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // decode protobuf list-of-keys response
    let bytes_res = resp.bytes().await;
    let bytes;
    match bytes_res {
        Ok(bytes_) => bytes = bytes_,
        Err(_e) => {
            assert!(false);
            panic!("silence E0381 warning");
        }
    }

    let in_msg;
    match DbStatResponse::parse_from_bytes(&bytes) {
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

async fn op_iter(client: &Client, db_id: String) {
    const DATA_COUNT: usize = 2001;
    let mut vdata: Vec<Vec<u8>> = Vec::new();

    // for simplicity, key==value in this test

    for i in 0..DATA_COUNT {
        let s = format!("datum {}", i);
        vdata.push(s.as_bytes().to_vec());
    }

    for s in &vdata {
        t_put_bytes(client, db_id.clone(), &s, &s).await;
    }

    let mut check_data: Vec<Vec<u8>> = Vec::new();

    let mut last_key: Option<Vec<u8>> = None;
    let mut list_end = false;
    while !list_end {
        let key_list = t_iter(client, db_id.clone(), last_key.clone()).await;

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

async fn op_get(client: &Client, db_id: String) {
    let test_key = String::from("op_key1");
    let test_value = format!("helloworld op_get {}", db_id);

    t_get_gone(client, db_id.clone(), test_key.clone()).await;
    t_put(client, db_id.clone(), test_key.clone(), test_value.clone()).await;
    t_get_ok(client, db_id.clone(), test_key.clone(), test_value).await;
    t_del(client, db_id.clone(), test_key.clone()).await;
    t_get_gone(client, db_id, test_key).await;
}

async fn op_clear(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let clear_url = format!("{}clear", basepath);
    let test_key = String::from("op_clear_key");
    let test_value = format!("helloworld op_clear {}", db_id);

    t_get_gone(client, db_id.clone(), test_key.clone()).await;
    t_put(client, db_id.clone(), test_key.clone(), test_value.clone()).await;
    t_get_ok(client, db_id.clone(), test_key.clone(), test_value).await;

    // exec clear-db request
    let resp_res = client.post(&clear_url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

    t_get_gone(client, db_id, test_key).await;
}

async fn op_put(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let put_url = format!("{}put", basepath);
    let get_url = format!("{}get", basepath);
    let del_url = format!("{}del", basepath);
    let test_key = String::from("op_put_key");
    let test_value = format!("helloworld op_put {}", db_id);

    // encode put request
    let out_bytes = pbenc_update_req(test_key.as_bytes(), test_value.as_bytes());

    // exec put request
    let resp_res = client.post(&put_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(_body) => {}
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // encode verification get request
    let out_bytes = pbenc_key_req(test_key.as_bytes());

    // exec get request
    let resp_res = client.post(&get_url).body(out_bytes.clone()).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // re-use same KeyRequest bytes for our delete request

    // exec del request
    let resp_res = client.post(&del_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }
}

async fn op_obj(client: &Client, db_id: String) {
    let basepath = format!("{}{}/{}/", T_ENDPOINT, T_BASEURI, db_id);
    let test_key = String::from("1");
    let test_value = format!("helloworld {}", db_id);

    // Check that a record with key 1 doesn't exist.
    let url = format!("{}obj/{}", basepath, test_key);
    let resp_res = client.get(&url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // verify DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // PUT a new record
    let resp_res = client.put(&url).body(test_value.clone()).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

    // Check that the record exists with the correct contents.
    let resp_res = client.get(&url).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // Check that the record exists with the correct contents,
    // protobuf-style.
    let out_bytes = pbenc_key_req(test_key.as_bytes());

    let get_pb_url = format!("{}get", basepath);
    let resp_res = client.post(&get_pb_url).body(out_bytes).send().await;
    match resp_res {
        Ok(resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text().await {
                Ok(body) => assert_eq!(body, test_value),
                Err(_e) => assert!(false),
            }
        }
        Err(_e) => assert!(false),
    }

    // DELETE record
    let resp_res = client.delete(&url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false),
    }

    // Check (again) that a record with key 1 doesn't exist.
    let resp_res = client.get(&url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }

    // verify (again) DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send().await;
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false),
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    // CLI parser static setup
    let cli_app = clap::App::new(APPNAME)
        .version(VERSION)
        .author("Jeff Garzik <jgarzik@pobox.com>")
        .about("Integration tester for kvdbd");

    // parse command line
    let _cli_matches = cli_app.get_matches();

    // create http client
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    // test, for each database
    for n in 1..3 {
        let db_id = format!("db{}", n);

        op_batch(&client, db_id.clone()).await;
        op_del(&client, db_id.clone()).await;
        op_get(&client, db_id.clone()).await;
        op_obj(&client, db_id.clone()).await;
        op_put(&client, db_id.clone()).await;
        op_clear(&client, db_id.clone()).await;
        op_stat(&client, db_id.clone()).await;
        op_iter(&client, db_id.clone()).await;
    }
    println!("Integration testing successful.");
    Ok(())
}
