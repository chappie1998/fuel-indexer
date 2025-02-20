![Fuel Logo](./img/fuel.png)

The Fuel indexer is a standalone service that can be used to index various components of the blockchain. These indexable components include blocks, transactions, receipts, and state within a Fuel network, allowing for high-performance read-only access to the blockchain for advanced dApp use-cases.

Events can be indexed using the Fuel indexer by creating WASM modules, as described in [the Hello World example](./src/examples/hello-indexer.md).

For more info on how to get started with Fuel Indexer, [read the book](https://fuellabs.github.io/fuel-indexer/latest/index.html).

## Documentation

### Building From Source

Install `mdbook` and then open a new terminal session in order to run the subsequent commands

```sh
cargo install mdbook
```

To build book:

```sh
mdbook build
```

To serve locally:

```sh
mdbook serve
```
