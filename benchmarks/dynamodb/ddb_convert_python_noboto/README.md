# DynamoDB JSON Converter

Reimplementation of [ddb_convert](../../examples/dynamodb/) in Python without `boto3` library.

Vibe coded and not reviewed, but good enough to convert Yelp dataset to and from DynamoDB JSON.

## Installation and usage

Install:

```
# No installation needed, just Python 3.6+
```

Use:

```
$ echo '{"name":"Alice","age":30}' | python3 ddb_convert.py to-ddb
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}

$ cat data.json
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}
$ python3 ddb_convert.py -i data.json from-ddb
{"name":"Alice","age":30}
```
