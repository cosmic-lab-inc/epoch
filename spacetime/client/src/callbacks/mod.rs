pub mod account;
pub mod connection;
pub mod message;
pub mod user;

pub use account::*;
pub use connection::*;
pub use message::*;
pub use user::*;

extern crate core;

use spacetimedb_sdk::{
    identity::once_on_connect,
    on_disconnect, on_subscription_applied, subscribe,
    table::{TableType, TableWithPrimaryKey},
};

use crate::reducers::{on_send_account, on_send_message, on_set_name};
use crate::tables::{Message, SpacetimeAccount, User};

/// Register all the callbacks our app will use to respond to database events.
pub fn register_callbacks() {
    // When we receive our `Credentials`, save them to a file.
    once_on_connect(on_connected);

    // When a new user joins, print a notification.
    User::on_insert(on_user_inserted);

    // When a user's status changes, print a notification.
    User::on_update(on_user_updated);

    // When a new message is received, print it.
    Message::on_insert(on_message_inserted);

    // When a account message is received, print it.
    SpacetimeAccount::on_insert(on_account_inserted);

    // When we receive the message backlog, print it in timestamp order.
    on_subscription_applied(on_sub_applied);

    // When we fail to set our name, print a warning.
    on_set_name(on_name_set);

    // When we fail to send a message, print a warning.
    on_send_message(on_message_sent);

    on_send_account(on_account_sent);

    // When our connection closes, inform the user and exit.
    on_disconnect(on_disconnected);
}

// TODO: source from env
pub const CREDS_DIR: &str = ".spacetime";

/// The URL of the SpacetimeDB instance hosting our chat module.
pub const SPACETIMEDB_URI: &str = "http://localhost:3000";

/// The module name we chose when we published our module.
pub const DB_NAME: &str = "epoch";
