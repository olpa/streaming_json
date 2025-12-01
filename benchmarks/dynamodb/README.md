## Benchmark transcript

Using:

- AWS instance type: `c6i.large`.
- Work in the suite `roundtrip_to_ddb`.
- Yelp academic dataset: download from https://business.yelp.com/data/resources/open-dataset/

Delete existing files in `original-normal`, unpack Yelp jsons there.

```
$ wc -l original-normal/*.json
    150346 original-normal/yelp_academic_dataset_business.json
    131930 original-normal/yelp_academic_dataset_checkin.json
   6990280 original-normal/yelp_academic_dataset_review.json
    908915 original-normal/yelp_academic_dataset_tip.json
   1987897 original-normal/yelp_academic_dataset_user.json
  10169368 total
```

Run "make clean" followed by "make check-eq" with a different "CONVTOOL", store the output in a "transcript/log-something" file.

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

Visualize, and check the stdout that stats make sense:

```
./visualize_performance.py
```
