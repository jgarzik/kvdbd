#![feature(proc_macro_hygiene,decl_macro,rustc_private)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;

#[cfg(test)] mod tests;

use std::sync::Mutex;
use std::collections::HashMap;

use serde_json::json;

use rocket::State;

type DbMap = Mutex<HashMap<String, String>>;

#[derive(Serialize, Deserialize)]
struct DbEntry {
    key: String,
    val: String
}

#[get("/<dbkey>")]
fn get(dbkey: String, map: State<'_, DbMap>) -> Option<String> {
    let hashmap = map.lock().unwrap();
    match hashmap.get(&dbkey) {
        Some(val) => Some(json!({"result": val}).to_string()),
        _ => None
    }
}

#[put("/<dbkey>/<dbval>")]
fn put(dbkey: String, dbval: String, map: State<'_, DbMap>) -> String {
    let mut hashmap = map.lock().unwrap();
    hashmap.insert(dbkey, dbval);

    json!({"result": true}).to_string()
}

#[catch(404)]
fn not_found() -> String {
    json!({"error": {
        "code": -404,
        "message": "URI not found"
        }}).to_string()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/1/db", routes![get, put])
        .register(catchers![not_found])
        .manage(Mutex::new(HashMap::<String, String>::new()))
}

fn main() {
    rocket().launch();
}
