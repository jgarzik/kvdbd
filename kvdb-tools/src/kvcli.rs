extern crate clap;

const APPNAME: &'static str = "kvcli";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const T_ENDPOINT: &'static str = "https://127.0.0.1:8080";

use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::{env, io};

use kvdb_lib::{client, pbapi};
use pbapi::{GetOp, GetRequest, KeyRequest, MutationRequest, UpdateRequest};
use protobuf::Message;

fn stdout_bytes(b: &[u8]) -> io::Result<()> {
    use std::os::unix::io::FromRawFd;
    let mut stdout: File;
    unsafe {
        stdout = File::from_raw_fd(1);
    }

    stdout.write_all(b)?;
    Ok(())
}

fn encode_get(key: String) -> io::Result<()> {
    let mut out_msg = KeyRequest::new();
    out_msg.key = key.as_bytes().to_vec();
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

fn encode_put(key: String, val: String) -> io::Result<()> {
    let mut out_msg = UpdateRequest::new();
    out_msg.key = key.as_bytes().to_vec();
    out_msg.value = val.as_bytes().to_vec();
    out_msg.is_insert = true;
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

fn encode_mget(batch_path: String) -> io::Result<()> {
    let file = File::open(batch_path)?;
    let mut reader = BufReader::new(file);

    let mut out_msg = GetRequest::new();

    loop {
        let mut line = String::new();
        let rc = reader.read_line(&mut line)?;
        if rc == 0 {
            break;
        }

        // Line 1: operation (query, ...)
        line = line.trim_end().to_string();
        match line.as_ref() {
            "q" => {
                // line 2: key
                let mut key = String::new();
                reader.read_line(&mut key)?;
                key = key.trim_end().to_string();

                let mut req = GetOp::new();
                req.key = key.as_bytes().to_vec();
                out_msg.ops.push(req);
            }
            _ => panic!("Invalid query op line"),
        }
    }

    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

fn encode_batch(batch_path: String) -> io::Result<()> {
    let file = File::open(batch_path)?;
    let mut reader = BufReader::new(file);

    let mut out_msg = MutationRequest::new();

    loop {
        let mut line = String::new();
        let rc = reader.read_line(&mut line)?;
        if rc == 0 {
            break;
        }

        // Line 1: operation (insert, remove, ...)
        line = line.trim_end().to_string();
        match line.as_ref() {
            "i" => {
                // line 2: key
                let mut key = String::new();
                reader.read_line(&mut key)?;
                key = key.trim_end().to_string();

                // line 3: value
                let mut value = String::new();
                reader.read_line(&mut value)?;
                value = value.trim_end().to_string();

                let mut req = UpdateRequest::new();
                req.key = key.as_bytes().to_vec();
                req.value = value.as_bytes().to_vec();
                req.is_insert = true;
                out_msg.reqs.push(req);
            }
            "r" => {
                // line 2: key
                let mut key = String::new();
                reader.read_line(&mut key)?;
                key = key.trim_end().to_string();

                let mut req = UpdateRequest::new();
                req.key = key.as_bytes().to_vec();
                req.is_insert = false;
                out_msg.reqs.push(req);
            }
            _ => panic!("Invalid batch op line"),
        }
    }

    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

async fn cmd_stat(endpoint: &str, db_id: &str) -> io::Result<()> {
    let mut kvdb_client = client::KvdbClient::new(endpoint.to_string(), db_id.to_string());
    let res = kvdb_client.stat().await;
    match res {
        None => Err(Error::new(ErrorKind::Other, "Database Stat Error")),
        Some(resp) => {
            println!("{:?}", resp);
            Ok(())
        }
    }
}

async fn cmd_get(endpoint: &str, db_id: &str, key: &str) -> io::Result<()> {
    let mut kvdb_client = client::KvdbClient::new(endpoint.to_string(), db_id.to_string());
    let res = kvdb_client.get1(key.to_string()).await;
    match res {
        None => Err(Error::new(
            ErrorKind::Other,
            "Error: Key not found in database.",
        )),
        Some(val) => stdout_bytes(&val),
    }
}

async fn cmd_put(endpoint: &str, db_id: &str, key: &str, value: &str) -> io::Result<()> {
    let mut kvdb_client = client::KvdbClient::new(endpoint.to_string(), db_id.to_string());
    let res = kvdb_client.put1(key.to_string(), value.to_string()).await;
    match res {
        false => Err(Error::new(
            ErrorKind::Other,
            "Error: Database store failed.",
        )),
        true => Ok(()),
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    // parse command line
    let op_vals = ["get", "mget", "del", "put", "mutate"];
    let cli_matches = clap::App::new(APPNAME)
        .version(VERSION)
        .about("Command line client for kvdbd")
        .arg(
            clap::Arg::with_name("stat")
                .long("stat")
                .help("Command: STAT - Database-wide stats")
                .required(false)
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("put")
                .long("put")
                .help("Command: PUT, based on --key and --value")
                .required(false)
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("get")
                .long("get")
                .help("Command: GET, based on --key")
                .required(false)
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("encode")
                .long("encode")
                .value_name("OP")
                .help("Command: ENCODE CLI args to protobuf output")
                .possible_values(&op_vals)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("endpoint")
                .long("endpoint")
                .value_name("ENDPOINT-URI")
                .help("HTTP or HTTPS endpoint for client connection to server")
                .default_value(T_ENDPOINT)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("dbid")
                .long("dbid")
                .value_name("DB-ID")
                .help("Database identifier for connection")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("key")
                .long("key")
                .value_name("KEY-CONTENT")
                .help("Key in a key/value pair")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("value")
                .long("value")
                .value_name("VALUE-CONTENT")
                .help("Value in a key/value pair")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("metadata")
                .long("metadata")
                .value_name("METADATA-INSTRUCTIONS-FILE")
                .help("Import stream of key/value get/put/del operations")
                .takes_value(true),
        )
        .get_matches();

    if cli_matches.is_present("decode") {
        Err(Error::new(
            ErrorKind::Other,
            "TODO: Decode not implemented yet",
        ))
    } else if cli_matches.is_present("encode") {
        let op = cli_matches.value_of("encode").unwrap();
        match op {
            "get" | "del" => {
                if !cli_matches.is_present("key") {
                    return Err(Error::new(ErrorKind::Other, "Missing --key"));
                }
                let key = cli_matches.value_of("key").unwrap();
                encode_get(key.to_string())
            }
            "mget" => {
                if !cli_matches.is_present("metadata") {
                    return Err(Error::new(ErrorKind::Other, "Missing --metadata"));
                }
                let batch_path = cli_matches.value_of("metadata").unwrap();
                encode_mget(batch_path.to_string())
            }
            "put" => {
                if !cli_matches.is_present("key") {
                    return Err(Error::new(ErrorKind::Other, "Missing --key"));
                }
                if !cli_matches.is_present("value") {
                    return Err(Error::new(ErrorKind::Other, "Missing --value"));
                }
                let key = cli_matches.value_of("key").unwrap();
                let val = cli_matches.value_of("value").unwrap();
                encode_put(key.to_string(), val.to_string())
            }
            "mutate" => {
                if !cli_matches.is_present("metadata") {
                    return Err(Error::new(ErrorKind::Other, "Missing --metadata"));
                }
                let batch_path = cli_matches.value_of("metadata").unwrap();
                encode_batch(batch_path.to_string())
            }
            _ => {
                panic!("Unhandled operation - should not happen");
            }
        }
    } else if cli_matches.is_present("put") {
        if !cli_matches.is_present("key")
            || !cli_matches.is_present("value")
            || !cli_matches.is_present("dbid")
        {
            return Err(Error::new(
                ErrorKind::Other,
                "Missing --key, --value or --dbid",
            ));
        }

        let endpoint = cli_matches.value_of("endpoint").unwrap();
        let dbid = cli_matches.value_of("dbid").unwrap();
        let key = cli_matches.value_of("key").unwrap();
        let value = cli_matches.value_of("value").unwrap();

        cmd_put(endpoint, dbid, key, value).await
    } else if cli_matches.is_present("get") {
        if !cli_matches.is_present("key") || !cli_matches.is_present("dbid") {
            return Err(Error::new(ErrorKind::Other, "Missing --key or --dbid"));
        }

        let endpoint = cli_matches.value_of("endpoint").unwrap();
        let dbid = cli_matches.value_of("dbid").unwrap();
        let key = cli_matches.value_of("key").unwrap();

        cmd_get(endpoint, dbid, key).await
    } else if cli_matches.is_present("stat") {
        if !cli_matches.is_present("dbid") {
            return Err(Error::new(ErrorKind::Other, "Missing --dbid"));
        }
        let endpoint = cli_matches.value_of("endpoint").unwrap();
        let dbid = cli_matches.value_of("dbid").unwrap();

        cmd_stat(endpoint, dbid).await
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Error: No command operation specified.",
        ))
    }
}
