# DynamoDB JSON Converter

Reimplementation of [ddb_convert](../../examples/dynamodb/) in Python.

Vibe coded and not reviewed, but good enough to convert Yelp dataset to and from DynamoDB JSON.

## Installation and usage

Install:

```
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

Use:

```
((venv)) $ echo '{"name":"Alice","age":30}' | ./ddb_convert.py to-ddb
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}

((venv)) $ cat data.json
{"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}
((venv)) $ ./ddb_convert.py -i data.json from-ddb
{"name":"Alice","age":30}
```
