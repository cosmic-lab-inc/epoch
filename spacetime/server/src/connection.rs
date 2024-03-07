use crate::tables::*;
use spacetimedb::{spacetimedb, ReducerContext};

#[spacetimedb(connect)]
// Called when a client connects to the SpacetimeDB
pub fn identity_connected(ctx: ReducerContext) {
    if let Some(user) = User::filter_by_identity(&ctx.sender) {
        // If this is a returning user, i.e. we already have a `User` with this `Identity`,
        // set `online: true`, but leave `name` and `identity` unchanged.
        User::update_by_identity(
            &ctx.sender,
            User {
                online: true,
                ..user
            },
        );
    } else {
        // If this is a new user, create a `User` row for the `Identity`,
        // which is online, but hasn't set a name.
        User::insert(User {
            name: None,
            identity: ctx.sender,
            online: true,
        })
        .unwrap();
    }
}

#[spacetimedb(disconnect)]
// Called when a client disconnects from SpacetimeDB
pub fn identity_disconnected(ctx: ReducerContext) {
    if let Some(user) = User::filter_by_identity(&ctx.sender) {
        User::update_by_identity(
            &ctx.sender,
            User {
                online: false,
                ..user
            },
        );
    } else {
        // This branch should be unreachable,
        // as it doesn't make sense for a client to disconnect without connecting first.
        log::warn!(
            "Disconnect event for unknown user with identity {:?}",
            ctx.sender
        );
    }
}
