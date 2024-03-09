extern crate core;

use spacetimedb_sdk::{identity::Identity, reducer::Status, Address};

use crate::module::ReducerEvent;
use crate::tables::User;

/// Our `User::on_insert` callback:
/// if the user is online, print a notification.
pub fn on_user_inserted(user: &User, _: Option<&ReducerEvent>) {
    if user.online {
        println!("User {} connected.", user_name_or_identity(user));
    }
}

pub fn user_name_or_identity(user: &User) -> String {
    user.name
        .clone()
        .unwrap_or_else(|| identity_leading_hex(&user.identity))
}

pub fn identity_leading_hex(id: &Identity) -> String {
    hex::encode(&id.bytes()[0..8])
}

/// Our `User::on_update` callback:
/// print a notification about name and status changes.
pub fn on_user_updated(old: &User, new: &User, _: Option<&ReducerEvent>) {
    if old.name != new.name {
        println!(
            "User {} renamed to {}.",
            user_name_or_identity(old),
            user_name_or_identity(new)
        );
    }
    if old.online && !new.online {
        println!("User {} disconnected.", user_name_or_identity(new));
    }
    if !old.online && new.online {
        println!("User {} connected.", user_name_or_identity(new));
    }
}

/// Our `on_set_name` callback: print a warning if the reducer failed.
pub fn on_name_set(
    _sender_id: &Identity,
    _sender_address: Option<Address>,
    status: &Status,
    name: &String,
) {
    if let Status::Failed(err) = status {
        eprintln!("Failed to change name to {:?}: {}", name, err);
    }
}
