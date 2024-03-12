<p align="center">
  <a href="https://cosmiclab.io">
    <img alt="Epoch" src="./assets/logo.png" width="250px" style="border-radius: 50%;"/>
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

## Local Development

Install cargo make to run pre-configured commands in `Makefile.toml`.
```shell
cargo install cargo-make
```

Install PostgreSQL to start database and create superuser
```shell
# For MacOS
cargo make install_postgresql_macos && cargo make start_postgresql_macos
# For Linux
cargo make install_postgresql_linux && cargo make start_postgresql_linux

# If getting the error: 
# psql: error: connection to server on socket "/tmp/.s.PGSQL.5432" failed: FATAL
# run this to debug:
rm /opt/homebrew/var/postgresql@13/postmaster.pid
brew services restart postgresql@13

# create supseruser
createuser -s postgres
# check that superuser exists
psql -U postgres -c "SELECT * FROM pg_user;"
psql -U postgres

# quit psql shell
\q
```

Initialize Postgres database and migrate tables
```shell
# reset and recreate database, create migrations, copy migrations to proper directory, and load migrations to database
cargo make init_database
```


### Start Backfill
Epoch reads the `backfill.yaml` config file which defines the snapshots to pull from Google Cloud Storage (GCS), the 
GCS bucket to pull from, the number of workers/threads to parallelize tasks, and the Solana programs to filter for.

Run:
```shell
cargo make backfill
```
With the default `backfill.yaml` you should see this output: `snapshot range: 66958784 - 251844968`


### Start Epoch Server
This requires the config yaml file `epoch.yaml` to be set. 
It needs the local path to the Google service account JSON.
The yaml file will look like this
```yaml
gcs_sa_key: epoch_sa_key.json
```

```shell
After running the backfill client to dump accounts into the Postgres database, you may run the Epoch server
```shell
cargo make epoch
```
The server defaults to http://localhost:3333 where you should see a welcome message.
The general API routes are behind `http://localhost:333/api`.

To see all accounts loaded by the backfill (careful is is a massive payload), 
check out http://localhost:3333/api/accounts.

Request body expects rust type `struct Paginate`. Use Insomnia or Postman to send a POST request to the server.
```json
{
  "limit": 10,
  "offset": 0
}
```
