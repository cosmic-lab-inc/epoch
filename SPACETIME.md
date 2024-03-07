
## Local Setup

1. [Install spacetime](https://spacetimedb.com/install)

2. Start server in one terminal
```shell
# running on localhost:3000
spacetime start 
```

3. Publish `module` project to local Spacetime instance
```shell
spacetime publish --project-path spacetime/module test-project
```

4. Send message from client
```shell
spacetime call test-project send_message 'Hello, World!' 
```

5. Print server logs to see message
```shell
spacetime logs test-project
```

6. Print db table to see Message row
```shell
spacetime sql test-project "SELECT * FROM Message"
```

7. In a separate terminal from the running server, run the client
```shell
cd client
cargo run
```
If you see `400 Bad Request` error, it's because the local spacetime instance was restarted and you did not 
run **Step 3** again. Publish the server package the server and try again.


## Wipe Local Setup
List identities of servers and copy the identity of the server you want to wipe
```shell
spacetime identity list
```

List databases associated with server identity
```shell
spacetime list <server-identity>
```

Copy db address you wish to delete and run the following:
```shell
spacetime delete <db-address>
```