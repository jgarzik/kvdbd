
#[macro_use] extern crate actix_web;
#[macro_use] extern crate serde_derive;

use std::{env, io};

use actix_web::http::{StatusCode};
use actix_web::{
    guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};
use serde_json::json;

use std::sync::Mutex;
use std::collections::HashMap;

type DbMap = web::Data<Mutex<HashMap<String, String>>>;

#[derive(Serialize, Deserialize)]
struct DbEntry {
    key: String,
    val: String
}

/// simple index handler
#[get("/")]
fn index(state: DbMap, req: HttpRequest) -> Result<HttpResponse> {
    println!("{:?}", req);

    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(json!({
            "name": "kvapp",
            "version": "0.1.0"}).to_string()))
}

/// GET data item
fn get(state: DbMap, req: HttpRequest, path: web::Path<(String,)>) -> Result<HttpResponse> {
    println!("{:?}", req);

    let hashmap = state.lock().unwrap();
    match hashmap.get(&path.0) {
        Some(val) => Ok(HttpResponse::build(StatusCode::OK)
                .content_type("application/json")
                .body(json!({"result": val}).to_string())),
        _ => Ok(HttpResponse::build(StatusCode::NOT_FOUND)
            .content_type("application/json")
            .body(json!({
              "error": {
                 "code" : -404,
                  "message": "not found"}}).to_string()))
    }
}

/// PUT data item
fn put(state: DbMap, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {
    println!("{:?}", req);

    let mut hashmap = state.lock().unwrap();
    hashmap.insert(path.0.clone(), path.1.clone());

    let bodystr = json!({"result": true}).to_string();

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(bodystr))
}

/// 404 handler
fn p404() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("application/json")
        .body(json!({
            "error": {
                "code" : -404,
                 "message": "not found"}}).to_string()))
}

fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();
    let sys = actix_rt::System::new("kvapp");
    let hmm = web::Data::new(Mutex::new(HashMap::<String, String>::new()));

    HttpServer::new(move || {
        App::new()
            .register_data(hmm.clone())
            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // register simple routes, handle all methods
            .service(index)
            .service(web::resource("/1/db/{dbkey}").route(web::get().to(get)))
            .service(web::resource("/1/db/{dbkey}/{dbval}").route(web::put().to(put)))
            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind("127.0.0.1:8080")?
    .start();

    println!("Starting http server: 127.0.0.1:8080");
    sys.run()
}
