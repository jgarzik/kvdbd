
#[macro_use] extern crate actix_web;
extern crate clap;

const APPNAME: &'static str = "kvdb";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_CFG_FN: &'static str = "cfg-kvdb.json";
const DEF_DB_NAME: &'static str = "db";
const DEF_DB_DIR: &'static str = "db.kv";
const DEF_BIND_ADDR: &'static str = "127.0.0.1";
const DEF_BIND_PORT: &'static str = "8080";

use std::{env, io, fs};
use std::sync::Mutex;

use actix_web::http::{StatusCode};
use actix_web::{
    guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::{Db,ConfigBuilder};

#[derive(Serialize, Deserialize)]
struct DbConfig {
    name:   String,
    path:   String
}

#[derive(Serialize, Deserialize)]
struct ServerConfig {
    databases:  Vec<DbConfig>
}

struct ServerState {
    name: String,       // db nickname
    db: Db              // open db handle
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

// helper function, server error
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
    println!("{:?}", req);

    let state = m_state.lock().unwrap();

    ok_json(json!({
        "name": APPNAME,
        "version": VERSION,
        "databases": [
            { "name": state.name }
        ]
    }))
}

/// DELETE data item.  key in URI path.  returned ok as json response
fn req_delete(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {
    println!("{:?}", req);

    let state = m_state.lock().unwrap();

    // we only support 1 db, for now...  user must specify db name
    if state.name != path.0 {
        return err_not_found();
    }

    match state.db.remove(path.1.clone()) {
        Ok(optval) => match optval {
            Some(_val) => ok_json(json!({"result": true})),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// GET data item.  key in URI path.  returned value as json response
fn req_get(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest, path: web::Path<(String,String)>) -> Result<HttpResponse> {
    println!("{:?}", req);

    let state = m_state.lock().unwrap();

    // we only support 1 db, for now...  user must specify db name
    if state.name != path.0 {
        return err_not_found();
    }

    match state.db.get(path.1.clone()) {
        Ok(optval) => match optval {
            Some(val) => ok_binary(val.to_vec()),
            None => err_not_found()     // db: value not found
        },
        Err(_e) => err_500()            // db: error
    }
}

/// PUT data item.  key and value both in URI path.
fn req_put(m_state: web::Data<Mutex<ServerState>>, req: HttpRequest,
           (path,body): (web::Path<(String,String)>,web::Bytes)) -> Result<HttpResponse> {
    println!("{:?}", req);

    let state = m_state.lock().unwrap();

    // we only support 1 db, for now...  user must specify db name
    if state.name != path.0 {
        return err_not_found();
    }

    match state.db.insert(path.1.as_str(), body.to_vec()) {
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

    // special case, until we have multiple dbs: find first db config, use it
    let db_name;
    let db_path;
    if server_cfg.databases.len() == 0 {
        db_name = String::from(DEF_DB_NAME);
        db_path = String::from(DEF_DB_DIR);
    } else {
        db_name = server_cfg.databases[0].name.clone();
        db_path = server_cfg.databases[0].path.clone();
    }

    // configure & open db
    let db_config = ConfigBuilder::new()
        .path(db_path)
        .use_compression(false)
        .build();
    let db = Db::start(db_config).unwrap();

    // configure web server
    let sys = actix_rt::System::new(APPNAME);

    HttpServer::new(move || {
        App::new()
            // pass application state to each handler
            .data(Mutex::new(ServerState {
                name: db_name.clone(),
                db: db.clone()
            }))

            // apply default headers
            .wrap(middleware::DefaultHeaders::new().header("Server", server_hdr.to_string()))

            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())

            // register our routes
            .service(req_index)
            .service(
                web::resource("/api/{db}/{key}")
                    .route(web::get().to(req_get))
                    .route(web::put().to(req_put))
                    .route(web::delete().to(req_delete))
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
