<p align="center">
  <a href="https://cosmiclab.io">
    <img alt="Epoch" src="./assets/logo.png" width="300px" style="border-radius: 50%;"/>
  </a>
</p>


<h1 align="center" style="font-size: 50px">
    Epoch ⏳
</h1>

Epoch improves upon Solana RPC infrastructure to provide a smoother user experience.

Solana RPC serves real-time **account** data only (historical transactions aren't meaningful in this context).
Epoch provides **historical account state** back to genesis.

Solana RPC serves raw accounts only, with the account data as an incomprehensible byte array.
Epoch provides **deserialized accounts** that are human-readable and what the end-user often wants.

Solana RPC serves simple requests for accounts, such as "get all accounts for the Jupiter program".
Epoch provides **complex queries** such as "get all accounts for the Jupiter program that have a balance greater 
than 1000 and an average positive account change over the past 60 days".

Solana RPCs are validators that filled in the role of data servers while the Solana ecosystem grew. 
Validators are designed to validatee blocks, not to store Terabytes of history and serve complex requests.

Solana created **Geyser** to let the validators push data to infrastructure that is better designed to 
store and serve it to end users.

This is Epoch. 

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

1. `spacetime/client/src/lib.rs` is a library that has callbacks that the user (you) can execute. 
See `SPACETIME.md` for commands to talk to the database and send `Hello, world!` messages.
You need to create some reducer/callback that does the same thing (sending a message) but it's the 
`SpacetimeAccount` rather than the `Message` as it is right now.
Then `epoch` can import that callback function and send every `SpacetimeAccount` to it.

2. Once the `SpacetimeAccount` is in a callback that `spacetime-client` can use, you need to write the function 
in `spacetime/client/src/main.rs` that uploads to the database in the `SpacetimeAccount` table.

3. Write some function that can be called the same way you can send the `Hello, world!` message in the `SPACETIME.md`
command guide. This function should fetch a `SpacetimeAccount` from the table by identity/key (account pubkey).
