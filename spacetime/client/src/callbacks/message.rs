extern crate core;

use crate::callbacks::user_name_or_identity;
use crate::module::ReducerEvent;
use crate::tables::{Message, User};
use spacetimedb_sdk::{identity::Identity, reducer::Status, Address};

/// Our `Message::on_insert` callback: print new messages.
pub fn on_message_inserted(message: &Message, reducer_event: Option<&ReducerEvent>) {
    if reducer_event.is_some() {
        print_message(message);
    }
}

pub fn print_message(message: &Message) {
    let sender = User::filter_by_identity(message.sender.clone())
        .map(|u| user_name_or_identity(&u))
        .unwrap_or_else(|| "unknown".to_string());
    println!("{}: {}", sender, message.text);
}

/// Our `on_send_message` callback: print a warning if the reducer failed.
pub fn on_message_sent(
    _sender_id: &Identity,
    _sender_address: Option<Address>,
    status: &Status,
    text: &String,
) {
    if let Status::Failed(err) = status {
        eprintln!("Failed to send message {:?}: {}", text, err);
    }
}
