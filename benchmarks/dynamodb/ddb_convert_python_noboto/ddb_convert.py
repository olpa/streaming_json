#!/usr/bin/env python3
"""Convert between DynamoDB JSON and normal JSON formats"""

import argparse
import json
import sys
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Tuple, Union, TextIO


class Mode(Enum):
    FROM_DDB = "from-ddb"
    TO_DDB = "to-ddb"


def detect_jsonl_from_content(file: TextIO) -> Tuple[bool, str]:
    """Detect if input is JSONL by trying to parse the first line"""
    first_line = file.readline()

    if not first_line.strip():
        return False, first_line

    try:
        json.loads(first_line.strip())
        return True, first_line
    except json.JSONDecodeError:
        return False, first_line


def unmarshall_value(value: Any) -> Any:
    """Convert a DynamoDB typed value to a normal value"""
    if not isinstance(value, dict):
        raise ValueError("Expected DynamoDB type object")

    if len(value) != 1:
        raise ValueError("DynamoDB type object must have exactly one key")

    type_key, type_value = next(iter(value.items()))

    if type_key == "S":
        return type_value
    elif type_key == "N":
        if not isinstance(type_value, str):
            raise ValueError("N type must be string")
        # Try to parse as int first, then float
        try:
            if '.' in type_value or 'e' in type_value.lower():
                return float(type_value)
            else:
                return int(type_value)
        except ValueError:
            raise ValueError(f"Invalid number format: {type_value}")
    elif type_key == "BOOL":
        return type_value
    elif type_key == "NULL":
        return None
    elif type_key == "M":
        if not isinstance(type_value, dict):
            raise ValueError("M type must be object")
        result = {}
        for k, v in type_value.items():
            result[k] = unmarshall_value(v)
        return result
    elif type_key == "L":
        if not isinstance(type_value, list):
            raise ValueError("L type must be array")
        return [unmarshall_value(item) for item in type_value]
    elif type_key == "SS":
        return type_value
    elif type_key == "NS":
        if not isinstance(type_value, list):
            raise ValueError("NS type must be array")
        result = []
        for item in type_value:
            if not isinstance(item, str):
                raise ValueError("NS items must be strings")
            try:
                if '.' in item or 'e' in item.lower():
                    result.append(float(item))
                else:
                    result.append(int(item))
            except ValueError:
                raise ValueError(f"Invalid number format: {item}")
        return result
    elif type_key == "BS":
        return type_value
    elif type_key == "B":
        return type_value
    else:
        raise ValueError(f"Unknown DynamoDB type: {type_key}")


def from_dynamodb(value: Any) -> Any:
    """Convert DynamoDB JSON format to normal JSON"""
    if not isinstance(value, dict):
        raise ValueError("Expected JSON object")

    obj = value.copy()

    # Check if it has "Item" wrapper
    if len(obj) == 1 and "Item" in obj:
        item_value = obj["Item"]
        if not isinstance(item_value, dict):
            raise ValueError("Expected Item to be an object")
        obj = item_value

    # Unmarshall DynamoDB format
    result = {}
    for key, val in obj.items():
        result[key] = unmarshall_value(val)

    return result


def marshall_value(value: Any) -> Dict[str, Any]:
    """Convert a normal value to DynamoDB typed value"""
    if value is None:
        return {"NULL": True}
    elif isinstance(value, bool):
        return {"BOOL": value}
    elif isinstance(value, (int, float)):
        return {"N": str(value)}
    elif isinstance(value, str):
        return {"S": value}
    elif isinstance(value, list):
        # Always use generic List type (L)
        items = [marshall_value(item) for item in value]
        return {"L": items}
    elif isinstance(value, dict):
        marshalled = {}
        for k, v in value.items():
            marshalled[k] = marshall_value(v)
        return {"M": marshalled}
    else:
        raise ValueError(f"Unsupported type: {type(value)}")


def to_dynamodb(value: Any, wrap_item: bool) -> Any:
    """Convert normal JSON to DynamoDB JSON format"""
    if not isinstance(value, dict):
        raise ValueError("Expected JSON object")

    # Marshall to DynamoDB format
    result = {}
    for key, val in value.items():
        result[key] = marshall_value(val)

    if wrap_item:
        return {"Item": result}
    else:
        return result


def process_jsonl(
    input_file: TextIO,
    output_file: TextIO,
    mode: Mode,
    pretty: bool,
    without_item: bool,
    first_line: str
) -> None:
    """Process JSONL input (one JSON object per line)"""
    # Process the first line that was already read during detection
    line = first_line.strip()
    if line:
        try:
            input_data = json.loads(line)
        except json.JSONDecodeError as e:
            raise ValueError(f"Invalid JSON on line 1: {e}")

        if mode == Mode.FROM_DDB:
            output_data = from_dynamodb(input_data)
        else:
            output_data = to_dynamodb(input_data, not without_item)

        if pretty:
            output_file.write(json.dumps(output_data, indent=2) + "\n")
        else:
            output_file.write(json.dumps(output_data) + "\n")

    # Process remaining lines
    for line_num, line in enumerate(input_file, start=2):
        line = line.strip()

        if not line:
            continue

        try:
            input_data = json.loads(line)
        except json.JSONDecodeError as e:
            raise ValueError(f"Invalid JSON on line {line_num}: {e}")

        if mode == Mode.FROM_DDB:
            output_data = from_dynamodb(input_data)
        else:
            output_data = to_dynamodb(input_data, not without_item)

        if pretty:
            output_file.write(json.dumps(output_data, indent=2) + "\n")
        else:
            output_file.write(json.dumps(output_data) + "\n")


def process_json(
    input_file: TextIO,
    output_file: TextIO,
    mode: Mode,
    pretty: bool,
    without_item: bool,
    first_line: str
) -> None:
    """Process regular JSON input"""
    content = first_line + input_file.read()

    try:
        input_data = json.loads(content)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON: {e}")

    if mode == Mode.FROM_DDB:
        output_data = from_dynamodb(input_data)
    else:
        output_data = to_dynamodb(input_data, not without_item)

    if pretty:
        output_file.write(json.dumps(output_data, indent=2) + "\n")
    else:
        output_file.write(json.dumps(output_data) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="ddb_convert",
        description="Convert between DynamoDB JSON and normal JSON formats"
    )

    parser.add_argument(
        "mode",
        choices=["from-ddb", "to-ddb"],
        help="Conversion mode"
    )
    parser.add_argument(
        "-i", "--input",
        type=Path,
        metavar="INPUT",
        help="Input file (stdin if not specified)"
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        metavar="OUTPUT",
        help="Output file (stdout if not specified)"
    )
    parser.add_argument(
        "-p", "--pretty",
        action="store_true",
        help="Pretty print output JSON"
    )
    parser.add_argument(
        "--without-item",
        action="store_true",
        help="Omit top-level 'Item' wrapper (only applies to to-ddb mode)"
    )

    args = parser.parse_args()
    mode = Mode.FROM_DDB if args.mode == "from-ddb" else Mode.TO_DDB

    try:
        # Open input
        if args.input:
            input_file = open(args.input, 'r', buffering=65536)
        else:
            input_file = sys.stdin

        try:
            # Detect if input is JSONL
            if args.input:
                # Check file extension first
                if args.input.suffix == ".jsonl":
                    is_jsonl = True
                    first_line = ""
                else:
                    is_jsonl, first_line = detect_jsonl_from_content(input_file)
            else:
                # For stdin, detect from content
                is_jsonl, first_line = detect_jsonl_from_content(input_file)

            # Open output
            if args.output:
                output_file = open(args.output, 'w', buffering=65536)
            else:
                output_file = sys.stdout

            try:
                if is_jsonl:
                    process_jsonl(input_file, output_file, mode, args.pretty, args.without_item, first_line)
                else:
                    process_json(input_file, output_file, mode, args.pretty, args.without_item, first_line)

                output_file.flush()
            finally:
                if args.output:
                    output_file.close()
        finally:
            if args.input:
                input_file.close()

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
