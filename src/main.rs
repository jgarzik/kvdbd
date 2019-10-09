
#[macro_use] extern crate actix_web;
extern crate clap;
mod protos;
mod db;

const APPNAME: &'static str = "kvdbd";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_CFG_FN: &'static str = "cfg-kvdbd.json";
const DEF_BIND_ADDR: &'static str = "127.0.0.1";
const DEF_BIND_PORT: &'static str = "8080";

use std::{env, io, fs, process};
use std::sync::{Arc,Mutex};
use std::collections::HashMap;

use actix_web::http::{StatusCode};
use actix_web::{
    guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
struct DbState {
    cfg: DbConfig,      // imported db configuration
    db: Box<dyn crate::db::api::Db + Send> // open db handle
}

// runtime server state info
struct ServerState {
    debug: bool,
    name_idx: HashMap<String,usize>,
    dbs: Vec<DbState>   // all open databases
}

struct Backend {
    cli_help: String,
    cli_value_name: String
}

struct BackendState {
    backends: HashMap<String,Backend>
}

fn build_backend(id: &str) -> Backend {
    let value_str = format!("{}-DB-PATH", id);
    let help_str = format!("Zeroconf; ignore server config, and create single database 'db' using backend {} with param {}", id, value_str);
    Backend {
        cli_help: help_str,
        cli_value_name: value_str
    }
}

fn register_backends() -> BackendState {
    let mut bs = BackendState { backends: HashMap::new() };

    bs.backends.insert(String::from("sled"), build_backend("sled"));

    return bs;
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
fn req_index(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest) -> Result<HttpResponse> {

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

/// DELETE data item. key in HTTP payload.  return ok as json response
fn req_del(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key, into KeyRequest struct
    let in_msg: KeyRequest;
    match parse_from_bytes::<KeyRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to remove record from db, based on key (path elem 1)
    match state.dbs[idx].db.del(in_msg.get_key()) {
        Ok(optval) => match optval {
            true => ok_json(json!({"result": true})),
            false => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// DELETE data item.  key in URI path.  return ok as json response
fn req_obj_delete(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to remove record from db, based on key (path elem 1)
    match state.dbs[idx].db.del(path.1.as_bytes()) {
        Ok(optval) => match optval {
            true => ok_json(json!({"result": true})),
            false => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// GET data item. key in URI path, returns value in HTTP payload.
fn req_obj_get(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {

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
    match state.dbs[idx].db.get(path.1.as_bytes()) {
        Ok(optval) => match optval {
            Some(val) => ok_binary(val.to_vec()),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// GET data item. key in HTTP payload, returns value in HTTP payload.
fn req_get(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest,
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

/// atomic PUT of multiple data items. data items in HTTP payload. ret json ok.
fn req_batch(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key/value pairs
    let in_msg: BatchRequest;
    match parse_from_bytes::<BatchRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }

    // build batch
    let mut batch = db::api::Batch::default();
    let updates = in_msg.reqs.to_vec();
    for update in &updates {
        if update.is_insert {
            batch.insert(&update.key, &update.value);
        } else {
            batch.remove(&update.key);
        }
    }

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key (path elem 1)
    match state.dbs[idx].db.apply_batch(&batch) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500()            // db: error
    }
}

/// PUT data item. key in URI path, value in HTTP payload.
fn req_obj_put(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest,
           (path,body): (web::Path<(String,String)>,web::Bytes)) -> Result<HttpResponse> {

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key (path elem 1)
    match state.dbs[idx].db.put(path.1.as_bytes(), &body.to_vec()) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500()            // db: error
    }
}

/// PUT data item. key/value in HTTP payload.
fn req_put(m_state: web::Data<Arc<Mutex<ServerState>>>, req: HttpRequest,
           (path,body): (web::Path<(String,)>,web::Bytes)) -> Result<HttpResponse> {

    // decode protobuf msg containing key, into KeyRequest struct
    let in_msg: UpdateRequest;
    match parse_from_bytes::<UpdateRequest>(&body) {
        Err(_e) => return err_bad_req(),
        Ok(req) => { in_msg = req; }
    }
    if !in_msg.is_insert { return err_bad_req(); }

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();
    if state.debug { println!("{:?}", req); }

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx
    }

    // attempt to store record in db, based on key
    match state.dbs[idx].db.put(&in_msg.key, &in_msg.value) {
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

    let backend_state = register_backends();

    // build cli help strings
    let help_config = format!("Sets a custom configuration file (default: {})", DEF_CFG_FN);
    let help_bind_addr = format!("Custom server socket bind address (default: {})", DEF_BIND_ADDR);
    let help_bind_port = format!("Custom server socket bind port (default: {})", DEF_BIND_PORT);

    // CLI parser static setup
    let mut cli_app = clap::App::new(APPNAME)
                      .version(VERSION)
                      .author("Jeff Garzik <jgarzik@pobox.com>")
                      .about("Database server for key/value db")
                      .arg(clap::Arg::with_name("config")
                           .short("c")
                           .long("config")
                           .value_name("JSON-FILE")
                           .help(&help_config)
                           .takes_value(true))
                      .arg(clap::Arg::with_name("bind-addr")
                           .long("bind-addr")
                           .value_name("IP-ADDRESS")
                           .help(&help_bind_addr)
                           .takes_value(true))
                      .arg(clap::Arg::with_name("bind-port")
                           .long("bind-port")
                           .value_name("PORT")
                           .help(&help_bind_port)
                           .takes_value(true));

    // CLI parser dynamic setup: add zeroconf database options
    for (be_name, be_info) in &backend_state.backends {
        cli_app = cli_app
                      .arg(clap::Arg::with_name(be_name)
                           .long(be_name)
                           .value_name(&be_info.cli_value_name)
                           .help(&be_info.cli_help)
                           .takes_value(true));
    }

    // parse command line
    let cli_matches = cli_app.get_matches();

    // configure based on CLI options
    let bind_addr = cli_matches.value_of("bind-addr").unwrap_or(DEF_BIND_ADDR);
    let bind_port = cli_matches.value_of("bind-port").unwrap_or(DEF_BIND_PORT);
    let bind_pair = format!("{}:{}", bind_addr, bind_port);
    let server_hdr = format!("{}/{}", APPNAME, VERSION);

    // init server state
    let mut name_idx = HashMap::new();
    let mut dbs = Vec::new();

    // determine if zeroconf is requested
    let mut zeroconf = false;
    let mut server_cfg = ServerConfig { debug: false, databases: vec![] };
    for (be_name, _be_info) in &backend_state.backends {

        // if matched, build single-db static configuration
        if cli_matches.is_present(be_name) {
            server_cfg = ServerConfig {
                debug: false,
                databases: vec![DbConfig {
                    name: String::from("db"),
                    path: cli_matches.value_of(be_name).unwrap().to_string(),
                    driver: be_name.clone(),
                    read_only: false
                }]
            };
            zeroconf = true;
            break;
        }
    }

    // read JSON configuration file, unless already configured
    if !zeroconf {
        let cfg_fn = cli_matches.value_of("config").unwrap_or(DEF_CFG_FN);
        let cfg_text = fs::read_to_string(cfg_fn)?;
        server_cfg = serde_json::from_str(&cfg_text)?;
    }

    // configure and open databases
    for db_cfg in &server_cfg.databases {

        // setup backend config
        let db_config = db::api::ConfigBuilder::new()
            .path(db_cfg.path.clone())
            .read_only(db_cfg.read_only)
            .build();

        // verify this is a known backend
        if !backend_state.backends.contains_key(&db_cfg.driver) {
            println!("config: Unsupported db driver {} specified.", db_cfg.driver);
            process::exit(1);
        }

	let driver = db::sled::new_driver();

        // add db to server state
        let next_idx = dbs.len();
        name_idx.insert(db_cfg.name.clone(), next_idx);
        dbs.push( DbState {
            cfg: db_cfg.clone(),
            db: driver.start_db(db_config).unwrap()
        });
    }

    let srv_state = Arc::new(Mutex::new(ServerState {
	    	debug: false,
		name_idx: name_idx,
		dbs: dbs
	    }));

    // configure web server
    let sys = actix_rt::System::new(APPNAME);

    HttpServer::new(move || {
        App::new()
            // pass application state to each handler
            .data(Arc::clone(&srv_state))

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
