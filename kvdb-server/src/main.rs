#[macro_use(get)]
extern crate actix_web;
extern crate clap;
extern crate openssl;

const APPNAME: &'static str = "kvdbd";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_CFG_FN: &'static str = "cfg-kvdbd.json";
const DEF_BIND_ADDR: &'static str = "127.0.0.1";
const DEF_BIND_PORT: &'static str = "8080";

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{env, fs, process};

use actix_web::http::StatusCode;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use protobuf::{EnumOrUnknown, Message};

use kvdb_lib::pbapi::{get_op_result, get_response, update_request, GetOpResult, GetResponse};
use kvdb_lib::{codec, db};

// struct used for both input (server config file) and output (server info)
#[derive(Serialize, Deserialize, Clone)]
struct DbConfig {
    name: String,
    path: String,
    driver: String,
    read_only: bool,
}

#[derive(Serialize, Deserialize)]
struct SslConfig {
    private_key_path: String, // empty, if no SSL
    cert_chain_path: String,  // empty, if no SSL
}

impl SslConfig {
    fn new() -> SslConfig {
        SslConfig {
            private_key_path: String::new(),
            cert_chain_path: String::new(),
        }
    }
}

// top-level schema for server configuration file
#[derive(Serialize, Deserialize)]
struct ServerConfig {
    debug: bool,
    ssl: SslConfig,
    databases: Vec<DbConfig>,
}

// top-level server info output struct
#[derive(Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
    databases: Vec<DbConfig>,
}

// JSON response to stat API request
#[derive(Serialize, Deserialize)]
struct DbStatResponseJson {
    n_records: String, // some JSON impl have trouble with big ints
}

// per-db runtime state info
struct DbState {
    cfg: DbConfig,                   // imported db configuration
    db: Box<dyn db::api::Db + Send>, // open db handle
}

// runtime server state info
#[allow(dead_code)]
struct ServerState {
    debug: bool,
    name_idx: HashMap<String, usize>,
    dbs: Vec<DbState>, // all open databases
}

struct Backend {
    cli_help: String,
    cli_value_name: String,
    driver: Box<dyn db::api::Driver>,
}

struct BackendState {
    backends: HashMap<String, Backend>,
}

fn build_backend(id: &str) -> Backend {
    let value_str = format!("{}-DB-PATH", id);
    let help_str = format!("Zeroconf; ignore server config, and create single database 'db' using backend {} with param {}", id, value_str);
    Backend {
        cli_help: help_str,
        cli_value_name: value_str,
        driver: match id {
            "sled" => db::sled::new_driver(),
            "lmdb" => db::lmdb::new_driver(),
            _ => panic!("unknown db driver"),
        },
    }
}

fn register_backends() -> BackendState {
    let mut bs = BackendState {
        backends: HashMap::new(),
    };

    bs.backends
        .insert(String::from("sled"), build_backend("sled"));
    bs.backends
        .insert(String::from("lmdb"), build_backend("lmdb"));

    return bs;
}

// helper function, 404 not found
fn err_not_found() -> HttpResponse {
    HttpResponse::build(StatusCode::NOT_FOUND)
        .content_type("application/json")
        .body(
            json!({
          "error": {
             "code" : -404,
              "message": "not found"}})
            .to_string(),
        )
}

// helper function, 400 bad request
fn err_bad_req() -> HttpResponse {
    HttpResponse::build(StatusCode::BAD_REQUEST)
        .content_type("application/json")
        .body(
            json!({
          "error": {
             "code" : -400,
              "message": "invalid/malformed request"}})
            .to_string(),
        )
}

// helper function, 500 server error
fn err_500() -> HttpResponse {
    HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
        .content_type("application/json")
        .body(
            json!({
          "error": {
             "code" : -500,
              "message": "internal server error"}})
            .to_string(),
        )
}

// helper function, success + binary response
fn ok_binary(val: Vec<u8>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(val)
}

// helper function, success + json response
fn ok_json(jval: serde_json::Value) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(jval.to_string())
}

/// simple root index handler, describes our service
#[get("/")]
async fn req_index(m_state: web::Data<Arc<Mutex<ServerState>>>) -> HttpResponse {
    // fill basic server info struct used for output
    let mut srv_info = ServerInfo {
        name: String::from(APPNAME),
        version: String::from(VERSION),
        databases: Vec::new(),
    };

    // lock runtime-live state data
    let state = m_state.lock().unwrap();

    // copy each db config into output struct
    for db_state in &state.dbs {
        srv_info.databases.push(db_state.cfg.clone());
    }

    // serialize structs into json
    let jv = serde_json::to_value(&srv_info).unwrap();

    // return json output
    ok_json(jv)
}

/// CLEAR all data items.
async fn req_clear(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    path: web::Path<(String,)>,
) -> HttpResponse {
    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to clear all records from db
    match state.dbs[idx].db.clear() {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500(), // db: error
    }
}

/// Return db stats as protobuf
async fn req_stat(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    path: web::Path<(String,)>,
) -> HttpResponse {
    // lock runtime-live state data
    let state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to list keys, starting at supplied key (or at db-start, if none)
    let res = state.dbs[idx].db.stat();
    if res.is_err() {
        return err_500();
    }
    let st = res.unwrap();

    // encode protobuf output to bytes
    let out_bytes = codec::pbenc_db_stat_resp(st.n_records);

    ok_binary(out_bytes)
}

/// Return db stats as JSON
async fn req_stat_json(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    path: web::Path<(String,)>,
) -> HttpResponse {
    // lock runtime-live state data
    let state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to list keys, starting at supplied key (or at db-start, if none)
    let res = state.dbs[idx].db.stat();
    if res.is_err() {
        return err_500();
    }
    let st = res.unwrap();

    // fill for-JSON-output struct with return data
    let out_msg = DbStatResponseJson {
        n_records: st.n_records.to_string(),
    };

    // serialize structs into json
    let jv = serde_json::to_value(&out_msg).unwrap();

    // return json output
    ok_json(jv)
}

/// Sequential iteration through all KEYS in db. Start-key in HTTP payload.
async fn req_iter(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    (path, body): (web::Path<(String,)>, web::Bytes),
) -> HttpResponse {
    // decode protobuf msg containing key, into KeyRequest struct
    let res = codec::pbdec_iter_req(&body);
    if res.is_none() {
        return err_bad_req();
    }
    let in_msg = res.unwrap();

    // lock runtime-live state data
    let state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to list keys, starting at supplied key (or at db-start, if none)
    let mut opts = db::api::IterOptions::new();
    if !in_msg.start_key.is_empty() {
        opts.start(&in_msg.start_key);
    }
    if !in_msg.prefix.is_empty() {
        opts.prefix(&in_msg.prefix);
    }
    let res = state.dbs[idx].db.iter_keys(opts);
    if res.is_err() {
        return err_500();
    }

    // encode protobuf output to bytes
    let key_list = res.unwrap();
    let out_bytes = codec::pbenc_iter_resp(&key_list);

    ok_binary(out_bytes)
}

/// DELETE data item. key in HTTP payload.  return ok as json response
async fn req_del(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    (path, body): (web::Path<(String,)>, web::Bytes),
) -> HttpResponse {
    // decode protobuf msg containing key, into KeyRequest struct
    let res = codec::pbdec_key_req(&body);
    if res.is_none() {
        return err_bad_req();
    }
    let in_msg = res.unwrap();

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to remove record from db, based on key (path elem 1)
    match state.dbs[idx].db.del(&in_msg.key) {
        Ok(optval) => match optval {
            true => ok_json(json!({"result": true})),
            false => err_not_found(), // db: value not found
        },
        Err(_e) => err_500(), // db: error
    }
}

/// Multiple-GET data item. key in HTTP payload, returns value in HTTP payload.
async fn req_mget(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    (path, body): (web::Path<(String,)>, web::Bytes),
) -> HttpResponse {
    // decode protobuf msg containing key, into KeyRequest struct
    let res = codec::pbdec_mget_req(&body);
    if res.is_none() {
        return err_bad_req();
    }
    let in_msg = res.unwrap();

    let mut out_msg = GetResponse::new();
    out_msg.magic = EnumOrUnknown::new(get_response::MagicNum::MAGIC);

    // lock runtime-live state data
    let state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    let ops = in_msg.ops.to_vec();
    for op in &ops {
        // attempt to read record from db, based on key (http payload)
        match state.dbs[idx].db.get(&op.key) {
            Ok(optval) => match optval {
                Some(val) => {
                    let mut out_res = GetOpResult::new();
                    if !op.skip_val {
                        out_res.val = val;
                    }
                    out_res.is_ok = true;
                    out_res.err = EnumOrUnknown::new(get_op_result::GetErr::NONE);
                    out_msg.res.push(out_res);
                }
                None => {
                    let mut out_res = GetOpResult::new();
                    out_res.is_ok = false;
                    out_res.err = EnumOrUnknown::new(get_op_result::GetErr::KEY_NOT_FOUND);
                    out_msg.res.push(out_res);
                }
            },
            Err(_e) => return err_500(), // db: error
        }
    }

    ok_binary(out_msg.write_to_bytes().unwrap())
}

/// atomic PUT of multiple data items. data items in HTTP payload. ret json ok.
async fn req_mutate(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    (path, body): (web::Path<(String,)>, web::Bytes),
) -> HttpResponse {
    // decode protobuf msg containing key/value pairs
    let res = codec::pbdec_mutate_req(&body);
    if res.is_none() {
        return err_bad_req();
    }
    let in_msg = res.unwrap();

    // build batch
    let mut batch = db::api::Batch::default();
    let updates = in_msg.reqs.to_vec();
    for update in &updates {
        if update.magic != EnumOrUnknown::new(update_request::MagicNum::MAGIC) {
            return err_bad_req();
        }
        if update.is_insert {
            batch.insert(&update.key, &update.value);
        } else {
            batch.remove(&update.key);
        }
    }

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to store record in db, based on key (path elem 1)
    match state.dbs[idx].db.apply_batch(&batch) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500(), // db: error
    }
}

/// PUT data item. key/value in HTTP payload.
async fn req_put(
    m_state: web::Data<Arc<Mutex<ServerState>>>,
    (path, body): (web::Path<(String,)>, web::Bytes),
) -> HttpResponse {
    // decode protobuf msg containing key, into KeyRequest struct
    let res = codec::pbdec_update_req(&body);
    if res.is_none() {
        return err_bad_req();
    }
    let in_msg = res.unwrap();
    if !in_msg.is_insert {
        return err_bad_req();
    }

    // lock runtime-live state data
    let mut state = m_state.lock().unwrap();

    // lookup database index by name (path elem 0)
    let idx: usize;
    match state.name_idx.get(&path.0) {
        None => return err_not_found(),
        Some(r_idx) => idx = *r_idx,
    }

    // attempt to store record in db, based on key
    match state.dbs[idx].db.put(&in_msg.key, &in_msg.value) {
        Ok(_optval) => ok_json(json!({"result": true})),
        Err(_e) => err_500(), // db: error
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    let backend_state = register_backends();

    // CLI parser static setup
    let mut cli_app = clap::App::new(APPNAME)
        .version(VERSION)
        .author("Jeff Garzik <jgarzik@pobox.com>")
        .about("Database server for key/value db")
        .arg(
            clap::Arg::with_name("config")
                .short('c')
                .long("config")
                .value_name("JSON-FILE")
                .help("Sets a custom configuration file")
                .default_value(DEF_CFG_FN)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("bind-addr")
                .long("bind-addr")
                .value_name("IP-ADDRESS")
                .help("Custom server socket bind address")
                .default_value(DEF_BIND_ADDR)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("bind-port")
                .long("bind-port")
                .value_name("PORT")
                .help("Custom server socket bind port")
                .default_value(DEF_BIND_PORT)
                .takes_value(true),
        );

    // CLI parser dynamic setup: add zeroconf database options
    for (be_name, be_info) in &backend_state.backends {
        cli_app = cli_app.arg(
            clap::Arg::new(&**be_name)
                .long(be_name)
                .value_name(&be_info.cli_value_name)
                .help(&*be_info.cli_help)
                .takes_value(true),
        );
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
    let mut server_cfg = ServerConfig {
        debug: false,
        ssl: SslConfig::new(),
        databases: vec![],
    };
    for (be_name, _be_info) in &backend_state.backends {
        // if matched, build single-db static configuration
        if cli_matches.is_present(be_name) {
            server_cfg = ServerConfig {
                debug: false,
                ssl: SslConfig::new(),
                databases: vec![DbConfig {
                    name: String::from("db"),
                    path: cli_matches.value_of(be_name).unwrap().to_string(),
                    driver: be_name.clone(),
                    read_only: false,
                }],
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

        let backend = &backend_state.backends[&db_cfg.driver];

        // add db to server state
        let next_idx = dbs.len();
        name_idx.insert(db_cfg.name.clone(), next_idx);
        dbs.push(DbState {
            cfg: db_cfg.clone(),
            db: backend.driver.start_db(db_config).unwrap(),
        });
    }

    let srv_state = Arc::new(Mutex::new(ServerState {
        debug: server_cfg.debug,
        name_idx: name_idx,
        dbs: dbs,
    }));

    // configure web server
    let app = move || {
        App::new()
            // pass application state to each handler
            .app_data(web::Data::new(Arc::clone(&srv_state)))
            // apply default headers
            .wrap(middleware::DefaultHeaders::new().add(("Server", server_hdr.to_string())))
            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // register our routes
            .service(req_index)
            .service(web::resource("/api/{db}/mutate").route(web::post().to(req_mutate)))
            .service(web::resource("/api/{db}/clear").route(web::post().to(req_clear)))
            .service(web::resource("/api/{db}/del").route(web::post().to(req_del)))
            .service(web::resource("/api/{db}/mget").route(web::post().to(req_mget)))
            .service(web::resource("/api/{db}/iter").route(web::post().to(req_iter)))
            .service(web::resource("/api/{db}/put").route(web::post().to(req_put)))
            .service(web::resource("/api/{db}/stat").route(web::get().to(req_stat)))
            .service(web::resource("/api/{db}/stat.json").route(web::get().to(req_stat_json)))
    };

    // if TLS key/cert present in config, run in TLS mode
    if !server_cfg.ssl.private_key_path.is_empty() && !server_cfg.ssl.cert_chain_path.is_empty() {
        let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(server_cfg.ssl.private_key_path, SslFiletype::PEM)
            .unwrap();
        builder
            .set_certificate_chain_file(server_cfg.ssl.cert_chain_path)
            .unwrap();
        println!("Starting https server: {}", bind_pair);
        HttpServer::new(app)
            .bind_openssl(bind_pair.to_string(), builder)?
            .run()
            .await

    // otherwise, plain ole HTTP
    } else {
        println!("Starting http server: {}", bind_pair);
        HttpServer::new(app)
            .bind(bind_pair.to_string())?
            .run()
            .await
    }
}
