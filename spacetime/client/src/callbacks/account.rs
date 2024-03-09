extern crate core;

use crate::module::ReducerEvent;
use crate::tables::{SpacetimeAccount, User};
use spacetimedb_sdk::{identity::Identity, reducer::Status, Address};

/// Our `Message::on_insert` callback: print new messages.
pub fn on_account_inserted(account: &SpacetimeAccount, reducer_event: Option<&ReducerEvent>) {
    if reducer_event.is_some() {
        print_account(account);
    }
}

pub fn print_account(account: &SpacetimeAccount) {
    println!("{:?}", account);
}

/// Our `on_send_message` callback: print a warning if the reducer failed.
pub fn on_account_sent(
    _sender_id: &Identity,
    _sender_address: Option<Address>,
    status: &Status,
    account: &SpacetimeAccount,
) {
    if let Status::Failed(err) = status {
        eprintln!("Failed to send account {:?}: {}", account, err);
    }
}
