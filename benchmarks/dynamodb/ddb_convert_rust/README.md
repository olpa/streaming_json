# DynamoDB JSON Converter

Reimplementation of [ddb_convert](../../examples/dynamodb/) in Rust without `scan_json` crate.

Vibe coded and not reviewed, but good enough to convert Yelp dataset to and from DynamoDB JSON.

## Installation and usage

Install:

```
cargo build --release
```

Use:

```
$ echo '{"name":"Alice","age":30}' | ./target/release/ddb_convert_rust to-ddb
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}

$ cat data.json
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}
$ ./target/release/ddb_convert_rust -i data.json from-ddb
{"name":"Alice","age":30}
```
