extern crate clap;

const APPNAME: &'static str = "kvdb-pb";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::{env, io, process};

use protobuf::Message;
include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use pbapi::{BatchRequest, KeyRequest, UpdateRequest};

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

fn encode_batch(batch_path: String) -> io::Result<()> {
    let file = File::open(batch_path)?;
    let mut reader = BufReader::new(file);

    let mut out_msg = BatchRequest::new();

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
                let mut key = String::new();
                reader.read_line(&mut key)?;
                key = key.trim_end().to_string();

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

fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // parse command line
    let op_vals = ["get", "del", "put", "batch"];
    let cli_matches = clap::App::new(APPNAME)
        .version(VERSION)
        .about("Wire protocol encode/decode for kvdbd")
        .arg(
            clap::Arg::from_usage("<op> 'The operation to decode/encode'")
                .possible_values(&op_vals)
                .required(true),
        )
        .arg(
            clap::Arg::with_name("decode")
                .short('d')
                .long("decode")
                .help("Decode protobuf input and print")
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("encode")
                .short('e')
                .long("encode")
                .help("Encode CLI args to protobuf output")
                .takes_value(false),
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
            clap::Arg::with_name("batch")
                .long("batch")
                .value_name("KEY-VALUE-FILE")
                .help("Import stream of key/value put/del mutations")
                .takes_value(true),
        )
        .get_matches();

    if cli_matches.is_present("decode") {
        println!("Decode not implemented yet."); // TODO
        process::exit(1);
    } else if cli_matches.is_present("encode") {
        let op = cli_matches.value_of("op").unwrap();
        match op {
            "get" | "del" => {
                if !cli_matches.is_present("key") {
                    println!("Missing --key");
                    process::exit(1);
                }
                let key = cli_matches.value_of("key").unwrap();
                return encode_get(key.to_string());
            }
            "put" => {
                if !cli_matches.is_present("key") {
                    println!("Missing --key");
                    process::exit(1);
                }
                if !cli_matches.is_present("value") {
                    println!("Missing --value");
                    process::exit(1);
                }
                let key = cli_matches.value_of("key").unwrap();
                let val = cli_matches.value_of("value").unwrap();
                return encode_put(key.to_string(), val.to_string());
            }
            "batch" => {
                if !cli_matches.is_present("batch") {
                    println!("Missing --batch");
                    process::exit(1);
                }
                let batch_path = cli_matches.value_of("batch").unwrap();
                return encode_batch(batch_path.to_string());
            }
            _ => {
                panic!("Unhandled operation - should not happen");
            }
        }
    } else {
        println!("Either --decode or --encode must be supplied.");
        process::exit(1);
    }
}
