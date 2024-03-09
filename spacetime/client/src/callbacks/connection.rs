extern crate core;

use crate::callbacks::{print_message, CREDS_DIR, DB_NAME, SPACETIMEDB_URI};
use crate::module::connect;
use crate::reducers::{send_message, set_name};
use crate::tables::Message;
use spacetimedb_sdk::table::TableType;
use spacetimedb_sdk::{
    identity::{load_credentials, save_credentials, Credentials},
    subscribe, Address,
};

/// Our `on_connect` callback: save our credentials to a file.
pub fn on_connected(creds: &Credentials, _client_address: Address) {
    if let Err(e) = save_credentials(CREDS_DIR, creds) {
        eprintln!("Failed to save credentials: {:?}", e);
    }
}

/// Our `on_subscription_applied` callback:
/// sort all past messages and print them in timestamp order.
pub fn on_sub_applied() {
    let mut messages = Message::iter().collect::<Vec<_>>();
    messages.sort_by_key(|m| m.sent);
    for message in messages {
        print_message(&message);
    }
}

/// Our `on_disconnect` callback: print a note, then exit the process.
pub fn on_disconnected() {
    eprintln!("Disconnected!");
    std::process::exit(0)
}

/// Load credentials from a file and connect to the database.
pub fn connect_to_db() {
    connect(
        SPACETIMEDB_URI,
        DB_NAME,
        load_credentials(CREDS_DIR).expect("Error reading stored credentials"),
    )
    .expect("Failed to connect");
}

/// Read each line of standard input, and either set our name or send a message as appropriate.
pub fn user_input_loop() {
    for line in std::io::stdin().lines() {
        let Ok(line) = line else {
            panic!("Failed to read from stdin.");
        };
        if let Some(name) = line.strip_prefix("/name ") {
            set_name(name.to_string());
        } else {
            send_message(line);
        }
    }
}

/// Register subscriptions for all rows of both tables.
pub fn subscribe_to_tables() {
    subscribe(&["SELECT * FROM User;", "SELECT * FROM Message;"]).unwrap();
}
