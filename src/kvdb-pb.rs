extern crate clap;
mod protos;

const APPNAME: &'static str = "kvdb-pb";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

use std::fs::File;
use std::io::Write;
use std::{env, io, process};

use protobuf::Message;
use protos::pbapi::{KeyRequest, UpdateRequest};

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
    out_msg.set_key(key.as_bytes().to_vec());
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

fn encode_put(key: String, val: String) -> io::Result<()> {
    let mut out_msg = UpdateRequest::new();
    out_msg.set_key(key.as_bytes().to_vec());
    out_msg.set_value(val.as_bytes().to_vec());
    out_msg.set_is_insert(true);
    let out_bytes: Vec<u8> = out_msg.write_to_bytes().unwrap();

    stdout_bytes(&out_bytes)
}

fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::init();

    // parse command line
    let op_vals = ["get", "del", "put"];
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
                .short("d")
                .long("decode")
                .help("Decode protobuf input and print")
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("encode")
                .short("e")
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
            _ => {}
        }
    } else {
        println!("Either --decode or --encode must be supplied.");
        process::exit(1);
    }

    Ok(())
}
