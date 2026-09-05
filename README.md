# Screeps Bot

## Requirements

Node.js and Rust are needed to run and deploy the code

```sh
npm i
cargo install wasm-pack
cargo install wasm-opt
```

## Configuration

Run `cp .example-screeps.yaml .screeps.yaml` to copy the config file

## Deploy

After setting up the config file, deploy with

```sh
npm run deploy -- --server mmo
```
