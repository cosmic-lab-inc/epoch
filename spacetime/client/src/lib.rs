extern crate core;

pub mod identity;
pub mod module;
pub mod reducers;
pub mod tables;

use crate::module::callbacks::*;

#[derive(Debug, Default)]
pub struct SpacetimeClient;
impl SpacetimeClient {
    pub fn new() -> SpacetimeClient {
        SpacetimeClient
    }

    /// Starts client that listens to user requests, Spacetime responses, events, and callbacks.
    pub fn run(&self) {
        register_callbacks();
        connect_to_db();
        subscribe_to_tables();
        user_input_loop();
    }
}
