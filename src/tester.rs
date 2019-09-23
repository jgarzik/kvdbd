
/*
 * tester: Integration tester for kvapp
 *
 * To be run separately from kvapp, assuming a clean and empty db:
 * $ cargo run --bin kvapp
 * $ cargo run --bin tester
 */

extern crate reqwest;

const T_ENDPOINT: &'static str = "http://127.0.0.1:8080";
const T_BASEURI: &'static str = "/1/db/";

use reqwest::{Client,StatusCode};

fn post_get_put_get() {
    let basepath = format!("{}{}", T_ENDPOINT, T_BASEURI);

    let client = Client::new();

    // Check that a record with key 1 doesn't exist.
    let url = format!("{}1", basepath);
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false)
    }

    // verify DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false)
    }

    // Add a new message with ID 1.
    let url = format!("{}1/helloworld", basepath);
    let resp_res = client.put(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false)
    }

    // Check that the record exists with the correct contents.
    let url = format!("{}1", basepath);
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(mut resp) => {
            assert_eq!(resp.status(), StatusCode::OK);

            match resp.text() {
                Ok(body) => assert_eq!(body, "helloworld"),
                Err(_e) => assert!(false)
            }
        }
        Err(_e) => assert!(false)
    }

    // DELETE record
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::OK),
        Err(_e) => assert!(false)
    }

    // Check (again) that a record with key 1 doesn't exist.
    let resp_res = client.get(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false)
    }

    // verify (again) DELETE(non exist) returns not-found
    let resp_res = client.delete(&url).send();
    match resp_res {
        Ok(resp) => assert_eq!(resp.status(), StatusCode::NOT_FOUND),
        Err(_e) => assert!(false)
    }
}

fn main() {
    post_get_put_get();
    println!("Integration testing successful.");
}

