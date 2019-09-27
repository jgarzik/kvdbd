
#[macro_use] extern crate actix_web;
extern crate clap;
mod protos;

const APPNAME: &'static str = "kvdb";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_CFG_FN: &'static str = "cfg-kvdb.json";
const DEF_BIND_ADDR: &'static str = "127.0.0.1";
const DEF_BIND_PORT: &'static str = "8080";

use std::{env, io, fs, process};
use std::sync::Mutex;
use std::collections::HashMap;

use actix_web::http::{StatusCode};
use actix_web::{
    guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::{Db,ConfigBuilder,Batch};

use protos::pbapi::{KeyRequest,BatchRequest,UpdateRequest};
use protobuf::{parse_from_bytes};

// struct used for both input (server config file) and output (server info)
#[derive(Serialize, Deserialize, Clone)]
struct DbConfig {
    name:   String,
    path:   String,
    driver: String,
    read_only: bool
}

// top-level schema for server configuration file
#[derive(Serialize, Deserialize)]
struct ServerConfig {
    debug:      bool,
    databases:  Vec<DbConfig>
}

// top-level server info output struct
#[derive(Serialize, Deserialize)]
struct ServerInfo {
    name:       String,
    version:    String,
    databases:  Vec<DbConfig>
}

// per-db runtime state info
#[derive(Clone)]
struct DbState {
    cfg: DbConfig,      // imported db configuration
    db: Db              // open db handle
}

// runtime server state info
#[derive(Clone)]
struct ServerState {
    debug: bool,
    name_idx: HashMap<String,usize>,
    dbs: Vec<DbState>   // all open databases
}

// helper function, 404 not found
fn err_not_found() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("application/json")
        .body(json!({
          "error": {
             "code" : -404,
              "message": "not found"}}).to_string()))
}

// helper function, 400 bad request
fn err_bad_req() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::BAD_REQUEST)
        .content_type("application/json")
        .body(json!({
          "error": {
             "code" : -400,
              "message": "invalid/malformed request"}}).to_string()))
}

// helper function, 500 server error
fn err_500() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
        .content_type("application/json")
        .body(json!({
          "error": {
             "code" : -500,
              "message": "internal server error"}}).to_string()))
}

// helper function, success + binary response
fn ok_binary(val: Vec<u8>) -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/octet-stream")
        .body(val))
}

// helper function, success + json response
fn ok_json(jval: serde_json::Value) -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(jval.to_string()))
}

/// simple root index handler, describes our service
#[get("/")]
fn req_index(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest) -> Result<HttpResponse> {

    // fill basic server info struct used for output
    let mut srv_info = ServerInfo {
        name: String::from(APPNAME),
        version: String::from(VERSION),
        databases: Vec::new()
    };

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // copy each db config into output struct
    for db_state in &state.dbs {
        srv_info.databases.push(db_state.cfg.clone());
    }

    // serialize structs into json
    let jv = serde_json::to_value(&srv_info)?;

    // return json output
    ok_json(jv)
}

/// DELETE data item. key in HTTP payload
fn req_del(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key, into KeyRequest struct
    let in_msg: KeyRequest;
    match parse_from_bytes::<KeyRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to remove record from db, based on key (path elem 1)
    match state.dbs[idx].db.remove(in_msg.get_key()) {
        Ok(optval) => match optval {
            Some(_val) => ok_json(json!({"result": true})),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// DELETE data item.  key in URI path.  returned ok as json response
fn req_obj_delete(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to remove record from db, based on key (path elem 1)
    match state.dbs[idx].db.remove(path.1.clone()) {
        Ok(optval) => match optval {
            Some(_val) => ok_json(json!({"result": true})),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// GET data item. key in URI path, value in HTTP payload.
fn req_obj_get(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to read record from db, based on key (path elem 1)
    match state.dbs[idx].db.get(path.1.clone()) {
        Ok(optval) => match optval {
            Some(val) => ok_binary(val.to_vec()),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// GET data item. key in HTTP payload
fn req_get(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key, into KeyRequest struct
    let in_msg: KeyRequest;
    match parse_from_bytes::<KeyRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to read record from db, based on key (http payload)
    match state.dbs[idx].db.get(in_msg.get_key()) {
        Ok(optval) => match optval {
            Some(val) => ok_binary(val.to_vec()),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// PUT multiple data items. protobuf-encoded key/value pairs in HTTP payload.
fn req_batch(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key/value pairs
    let in_msg: BatchRequest;
    match parse_from_bytes::<BatchRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }

    // build sled batch
    let mut batch = Batch::default();
    let updates = in_msg.reqs.to_vec();
    for update in &updates {
        if update.is_insert {
            batch.insert(update.key.clone(), update.value.clone());
        } else {
            batch.remove(update.key.clone());
        }
    }

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key (path elem 1)
    match state.dbs[idx].db.apply_batch(batch) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500()            // db: error
    }
}

/// PUT data item. key in URI path, value in HTTP payload.
fn req_obj_put(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,String)>,web::Bytes)) -> Result<HttpResponse> {

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key (path elem 1)
    match state.dbs[idx].db.insert(path.1.as_str(), body.to_vec()) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500()            // db: error
    }
}

/// PUT data item. key/value in HTTP payload.
fn req_put(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key, into KeyRequest struct
    let in_msg: UpdateRequest;
    match parse_from_bytes::<UpdateRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }
    if !in_msg.is_insert { return err_bad_req(); }

    // lock runtime-live state data
    let state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key
    match state.dbs[idx].db.insert(in_msg.key, in_msg.value) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500()            // db: error
    }
}

/// 404 handler
fn p404() -> Result<HttpResponse> {
    err_not_found()
}

fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // parse command line
    let cli_matches = clap::App::new(APPNAME)
                      .version(VERSION)
                      .author("Jeff Garzik <jgarzik@pobox.com>")
                      .about("Database server for key/value db")
                      .arg(clap::Arg::with_name("config")
                           .short("c")
                           .long("config")
                           .value_name("JSON-FILE")
                           .help(&format!("Sets a custom configuration file (default: {})", DEF_CFG_FN))
                           .takes_value(true))
                      .arg(clap::Arg::with_name("bind-addr")
                           .long("bind-addr")
                           .value_name("IP-ADDRESS")
                           .help(&format!("Custom server socket bind address (default: {})", DEF_BIND_ADDR))
                           .takes_value(true))
                      .arg(clap::Arg::with_name("bind-port")
                           .long("bind-port")
                           .value_name("PORT")
                           .help(&format!("Custom server socket bind port (default: {})", DEF_BIND_PORT))
                           .takes_value(true))
                      .get_matches();

    // configure based on CLI options
    let bind_addr = cli_matches.value_of("bind-addr").unwrap_or(DEF_BIND_ADDR);
    let bind_port = cli_matches.value_of("bind-port").unwrap_or(DEF_BIND_PORT);
    let bind_pair = format!("{}:{}", bind_addr, bind_port);
    let server_hdr = format!("{}/{}", APPNAME, VERSION);

    // read JSON configuration file
    let cfg_fn = cli_matches.value_of("config").unwrap_or(DEF_CFG_FN);
    let cfg_text = fs::read_to_string(cfg_fn)?;
    let server_cfg: ServerConfig = serde_json::from_str(&cfg_text)?;

    // init server state
    let mut srv_state = ServerState {
        debug: server_cfg.debug,
        name_idx: HashMap::new(),
        dbs: Vec::new()
    };

    // configure and open databases
    for db_cfg in &server_cfg.databases {
        let db_config = ConfigBuilder::new()
            .path(db_cfg.path.clone())
            .use_compression(false)
            .read_only(db_cfg.read_only)
            .build();

        if db_cfg.driver != "sled".to_string() {
            println!("config: Unsupported db driver {} specified.", db_cfg.driver);
            process::exit(1);
        }

        let next_idx = srv_state.dbs.len();
        srv_state.name_idx.insert(db_cfg.name.clone(), next_idx);
        srv_state.dbs.push( DbState {
            cfg: db_cfg.clone(),
            db: Db::start(db_config).unwrap()
        });
    }

    // configure web server
    let sys = actix_rt::System::new(APPNAME);

    HttpServer::new(move || {
        App::new()
            // pass application state to each handler
            .data(Mutex::new(srv_state.clone()))

            // apply default headers
            .wrap(middleware::DefaultHeaders::new().header("Server", server_hdr.to_string()))

            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())

            // register our routes
            .service(req_index)
            .service(
                web::resource("/api/{db}/batch")
                    .route(web::post().to(req_batch))
            )
            .service(
                web::resource("/api/{db}/del")
                    .route(web::post().to(req_del))
            )
            .service(
                web::resource("/api/{db}/get")
                    .route(web::post().to(req_get))
            )
            .service(
                web::resource("/api/{db}/obj/{key}")
                    .route(web::get().to(req_obj_get))
                    .route(web::put().to(req_obj_put))
                    .route(web::delete().to(req_obj_delete))
            )
            .service(
                web::resource("/api/{db}/put")
                    .route(web::post().to(req_put))
            )

            // default
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET` -- redundant?
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind(bind_pair.to_string())?
    .start();

    println!("Starting http server: {}", bind_pair);
    sys.run()
}
