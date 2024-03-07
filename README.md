<p align="center">
  <a href="https://cosmiclab.io">
    <img alt="Epoch" src="./assets/logo.png" width="300px" style="border-radius: 50%;"/>
  </a>
</p>


# Epoch


## Development

### Start Spacetime Instance (server/database)
For more spacetime information see `SPACETIME.md`.
Start spacetime instance:
```shell
spacetime start
```
Deploy spacetime module to instance:
```shell
spacetime publish --project-path spacetime/server test-project
```

### Start Backfill
Epoch reads the `backfill.yaml` config file which defines the snapshots to pull from Google Cloud Storage (GCS), the 
GCS bucket to pull from, the number of workers/threads to parallelize tasks, and the Solana programs to filter for.

Run:
```shell
cargo make backfill
```
With the default `backfill.yaml` you should see this output: `snapshot range: 66958784 - 251844968`



## TODO:

1. `spacetime/client/src/lib.rs` has `SpacetimeAccount`. Cast the `ArchiveAccount` epoch gets from the snapshot into 
`SpacetimeAccount` so spacetime can use it.

1.`spacetime/client/src/lib.rs` is a library that has callbacks that the user (you) can execute. 
See `SPACETIME.md` for commands to talk to the database and send `Hello, world!` messages.
You need to create some reducer/callback that does the same thing (sending a message) but it's the 
`SpacetimeAccount` rather than the `Message` as it is right now.
Then `epoch` can import that callback function and send every `SpacetimeAccount` to it.

2. Once the `SpacetimeAccount` is in a callback that `spacetime-client` can use, you need to write the function 
in `spacetime/client/src/main.rs` that uploads to the database in the `SpacetimeAccount` table.

3. Write some function that can be called the same way you can send the `Hello, world!` message in the `SPACETIME.md`
command guide. This function should fetch a `SpacetimeAccount` from the table by identity/key (account pubkey).
