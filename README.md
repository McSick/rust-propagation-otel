# Distributed tracing example of a rust client and server using tracing crate and opentelemetry

This example demonstrates how to use the `tracing` crate and `opentelemetry` to instrument a simple client and server in Rust.

## How to run

1. Start the server:

```sh
$ cd rust-server
$ cargo run --bin dice_server
```

2. In another terminal, run the client:

```sh
$ cd rust-client
$ cargo run --bin dice_client
```

The client is a simple program that asks you to guess a dice roll. The server will respond with the actual roll and whether you guessed correctly.

Example Trace
![Trace View](example_trace.png?raw=true "Trace View")