//
// tests/integration.rs -- basic API end-to-end integration testing
//
// Copyright (c) 2024 Jeff Garzik
//
// This file is part of the pcgtoolssoftware project covered under
// the MIT License.  For the full license text, please see the LICENSE
// file in the root directory of this project.
// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

const DEF_START_WAIT: u64 = 4;
const T_VALUE: &'static str = "helloworld";

// A utility function to prepare the environment before starting the server.
fn prepare_environment() {
    // Use CARGO_MANIFEST_DIR to get the path to the source directory
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Construct the source path for the configuration file
    let config_src_path = Path::new(&cargo_dir).join("example-cfg-kvdbd.json");
    // Define the destination path for the configuration file
    let config_dest_path = Path::new("cfg-kvdbd.json");

    // Copy the configuration file to the current directory.
    fs::copy(config_src_path, config_dest_path).expect("Failed to copy configuration file");

    const DB_DIRS: &[&str] = &["db1.kv", "db2.kv"];

    for dirpath in DB_DIRS {
        // Create the db*.kv directory if it does not exist.
        let db_dir = Path::new(dirpath);
        if !db_dir.exists() {
            fs::create_dir(db_dir).expect("Failed to create directory");
        }
    }
}

// A utility function to start the kvdbd server.
// Returns a Child process handle, which can be used to kill the server later.
fn start_kvdbd_server() -> Child {
    // Specify the binary name using "--bin kvdbd" parameter to `cargo run`.
    let child = Command::new("cargo")
        .args(["run", "--bin", "kvdbd"])
        .spawn()
        .expect("Failed to start kvdbd server");

    // Give the server some time to start up.
    thread::sleep(Duration::from_secs(DEF_START_WAIT));

    child
}

// A utility function to stop the kvdbd server.
fn stop_kvdbd_server(mut child: Child) {
    child.kill().expect("Failed to kill kvdbd server");
}

// Example of an integration test that starts the server, makes a request, and stops the server.
#[tokio::test]
async fn test_kvdbd_integration() {
    // Prepare server environment
    prepare_environment();

    // Start the server in the background.
    let server_process = start_kvdbd_server();

    // Create HTTP client
    let client = reqwest::Client::new();

    // Stop the server.
    stop_kvdbd_server(server_process);
}
