# Benchmark based on DynamoDB JSON format conversion

About DynamoDB format: See [`scan_json` DynamoDB example](../../examples/dynamodb).

Contents:

- `ddb_convert_rust`: reimplementation of the DynamoDB example in Rust using `json_serde`, mapping JSON records to memory dictionaries, and transforming dictionaries between formats
- `ddb_convert_python_noboto`: similar to `ddb_convert_rust` but implemented in Python
- `ddb_convert_python`: uses the `boto3.dynamodb` library for conversion
- `roundtrip_from_ddb`: starts with a fixture in the DynamoDB format, converts it to normal JSON, then converts back to DynamoDB format, and checks that the result equals the original JSON
- `roundtrip_to_ddb`: similar to `roundtrip_from_ddb`, but starts with the normal JSON format
- `json-eq.sh`: tool to semantically compare JSON files

## Results

`scan_json` is the fastest, outperforming the Python boto version by twelve times.

![performance plot](./transcript/performance_comparison.png)

## Benchmark transcript

Using:

- AWS instance type: `c6i.large`
- Working in the suite `roundtrip_to_ddb`
- Yelp academic dataset: download from https://business.yelp.com/data/resources/open-dataset/

Delete existing files in `original-normal`, unpack Yelp JSONs there.

```
$ wc -l original-normal/*.json
    150346 original-normal/yelp_academic_dataset_business.json
    131930 original-normal/yelp_academic_dataset_checkin.json
   6990280 original-normal/yelp_academic_dataset_review.json
    908915 original-normal/yelp_academic_dataset_tip.json
   1987897 original-normal/yelp_academic_dataset_user.json
  10169368 total
```

Run `make clean` followed by `make check-eq` with a different `CONVTOOL`, and store the output in a `transcript/log-something` file.

Summarize logs:

```
>stats.json
./parse_logs.py log-py-boto py-boto >>stats.json
./parse_logs.py log-py-noboto py-noboto >>stats.json
./parse_logs.py log-rust rust >>stats.json
./parse_logs.py log-scan-json scan-json >>stats.json
```

Add file stats:

```
./file_stats.py ......./yelp_academic_dataset_*.json >file_stats.json
```

Visualize, and check that the stats in stdout make sense:

```
./visualize_performance.py
```

Result is in `performance_comparison.png`.
