#!/usr/bin/env python3
"""
ddb_convert - Convert between DynamoDB JSON format and normal JSON format

Usage:
    ddb_convert.py from-ddb [-i/--input <input_file>] [-o/--output <output_file>] [-p/--pretty]
    ddb_convert.py to-ddb [-i/--input <input_file>] [-o/--output <output_file>] [-p/--pretty] [--without-item]
    ddb_convert.py --help

Commands:
    from-ddb    Convert DynamoDB JSON to normal JSON format
    to-ddb      Convert normal JSON to DynamoDB JSON format

Options:
    -i/--input <file>    Input file path (default: stdin)
    -o/--output <file>   Output file path (default: stdout)
    -p/--pretty          Pretty print output JSON
    --without-item       Omit top-level "Item" wrapper (only applies to to-ddb mode)
    -h/--help            Show this help message

Notes:
    - Supports both single JSON objects and JSONL (one JSON object per line)
    - When converting to DynamoDB format, output is wrapped in {"Item": {...}} by default
    - Use --without-item to omit the wrapper
"""

import base64
import json
import sys
import argparse
import io
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, TextIO

try:
    from boto3.dynamodb.types import TypeSerializer, TypeDeserializer, Binary
except ImportError:
    print("Error: boto3 is required. Install it with: pip install boto3", file=sys.stderr)
    sys.exit(1)


def make_json_serializable(obj: Any) -> Any:
    """
    Recursively convert Python objects to JSON-serializable types.
    Handles boto3's output types: set, bytes, Decimal, Binary
    """
    if isinstance(obj, set):
        # Convert set to list, recursively processing items
        return [make_json_serializable(item) for item in obj]
    elif isinstance(obj, (bytes, Binary)):
        # Convert bytes/Binary to base64 string
        if isinstance(obj, Binary):
            obj = bytes(obj)  # Convert Binary to bytes
        return base64.b64encode(obj).decode('utf-8')
    elif isinstance(obj, Decimal):
        # Convert Decimal to int or float, preserving original intent
        # Check string representation: "4" -> int, "4.0" -> float
        str_repr = str(obj)
        if '.' in str_repr or 'e' in str_repr.lower():
            return float(obj)
        else:
            return int(obj)
    elif isinstance(obj, dict):
        return {k: make_json_serializable(v) for k, v in obj.items()}
    elif isinstance(obj, (list, tuple)):
        return [make_json_serializable(item) for item in obj]
    else:
        return obj


class DynamoDBJSONConverter:
    """Converter between DynamoDB JSON format and normal JSON format using boto3"""

    def __init__(self):
        self.serializer = TypeSerializer()
        self.deserializer = TypeDeserializer()

    def marshall(self, python_obj: Dict[str, Any]) -> Dict[str, Any]:
        """Convert a standard dict into DynamoDB format using boto3's TypeSerializer"""
        return {k: self.serializer.serialize(v) for k, v in python_obj.items()}

    def unmarshall(self, dynamo_obj: Dict[str, Any]) -> Dict[str, Any]:
        """Convert a DynamoDB dict into standard dict using boto3's TypeDeserializer"""
        # Preprocess to convert B field string values to bytes if needed
        preprocessed = self._preprocess_binary_fields(dynamo_obj)
        return {k: self.deserializer.deserialize(v) for k, v in preprocessed.items()}

    def _preprocess_binary_fields(self, obj: Any) -> Any:
        """Convert base64 string in B fields to bytes for boto3 compatibility"""
        if isinstance(obj, dict):
            if "B" in obj and isinstance(obj["B"], str):
                # Convert base64 string to bytes
                return {"B": base64.b64decode(obj["B"])}
            else:
                # Recursively process nested structures
                return {k: self._preprocess_binary_fields(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._preprocess_binary_fields(item) for item in obj]
        else:
            return obj

    def to_dynamodb(self, obj: Any, wrap_item: bool = True) -> Dict[str, Any]:
        """Convert normal JSON object to DynamoDB JSON format"""
        # Convert floats to Decimal as required by boto3
        obj = self._convert_floats_to_decimal(obj)
        if isinstance(obj, dict):
            # Handle as a DynamoDB item
            result = self.marshall(obj)
            if wrap_item:
                return {"Item": result}
            return result
        else:
            # Handle as a single value
            return self.serializer.serialize(obj)

    def _convert_floats_to_decimal(self, obj: Any) -> Any:
        """Recursively convert float values to Decimal for boto3 compatibility"""
        if isinstance(obj, float):
            return Decimal(str(obj))
        elif isinstance(obj, dict):
            return {k: self._convert_floats_to_decimal(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._convert_floats_to_decimal(item) for item in obj]
        else:
            return obj

    def from_dynamodb(self, dynamodb_obj: Dict[str, Any]) -> Dict[str, Any]:
        """Convert DynamoDB JSON format to normal JSON object"""
        # Check if it has "Item" wrapper and unwrap it
        if "Item" in dynamodb_obj and len(dynamodb_obj) == 1:
            dynamodb_obj = dynamodb_obj["Item"]

        # Unmarshall the DynamoDB item
        return self.unmarshall(dynamodb_obj)


def safe_json_dumps(obj: Any, pretty: bool = False, ensure_ascii: bool = False) -> str:
    """
    Safely serialize object to JSON string, applying cleanup if needed.
    Tries direct serialization first, and if it fails, applies make_json_serializable.
    """
    try:
        # Try direct serialization
        if pretty:
            return json.dumps(obj, indent=2, ensure_ascii=ensure_ascii)
        else:
            return json.dumps(obj, ensure_ascii=ensure_ascii, separators=(',', ':'))
    except (TypeError, ValueError):
        # If serialization fails, apply cleanup and try again
        cleaned_obj = make_json_serializable(obj)
        if pretty:
            return json.dumps(cleaned_obj, indent=2, ensure_ascii=ensure_ascii)
        else:
            return json.dumps(cleaned_obj, ensure_ascii=ensure_ascii, separators=(',', ':'))


def process_jsonl(input_stream: TextIO, output_stream: TextIO, converter: DynamoDBJSONConverter,
                  mode: str, pretty: bool, without_item: bool, first_line: str = "") -> None:
    """Process JSONL input line by line"""
    line_num = 0

    # Process the first line that was already read during detection
    if first_line.strip():
        line_num = 1
        try:
            # Parse the JSON line
            input_data = json.loads(first_line)

            # Convert based on mode
            if mode == 'to-ddb':
                output_data = converter.to_dynamodb(input_data, wrap_item=not without_item)
            else:  # from-ddb
                output_data = converter.from_dynamodb(input_data)

            # Output the result - safe_json_dumps handles non-serializable types
            json_str = safe_json_dumps(output_data, pretty=pretty, ensure_ascii=False)
            output_stream.write(json_str + '\n')

        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON on line {line_num}: {e}", file=sys.stderr)
            sys.exit(1)
        except Exception as e:
            print(f"Error processing line {line_num}: {e}", file=sys.stderr)
            sys.exit(1)

    # Process remaining lines
    for line in input_stream:
        line_num += 1
        line = line.strip()
        if not line:
            # Skip empty lines
            continue

        try:
            # Parse the JSON line
            input_data = json.loads(line)

            # Convert based on mode
            if mode == 'to-ddb':
                output_data = converter.to_dynamodb(input_data, wrap_item=not without_item)
            else:  # from-ddb
                output_data = converter.from_dynamodb(input_data)

            # Output the result - safe_json_dumps handles non-serializable types
            json_str = safe_json_dumps(output_data, pretty=pretty, ensure_ascii=False)
            output_stream.write(json_str + '\n')

        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON on line {line_num}: {e}", file=sys.stderr)
            sys.exit(1)
        except Exception as e:
            print(f"Error processing line {line_num}: {e}", file=sys.stderr)
            sys.exit(1)


def process_json(input_stream: TextIO, output_stream: TextIO, converter: DynamoDBJSONConverter,
                 mode: str, pretty: bool, without_item: bool, first_line: str = "") -> None:
    """Process single JSON object (non-JSONL)"""
    try:
        # Read entire input as single JSON
        content = first_line + input_stream.read()
        input_data = json.loads(content)

        # Convert based on mode
        if mode == 'to-ddb':
            output_data = converter.to_dynamodb(input_data, wrap_item=not without_item)
        else:  # from-ddb
            output_data = converter.from_dynamodb(input_data)

        # Output the result - safe_json_dumps handles non-serializable types
        json_str = safe_json_dumps(output_data, pretty=pretty, ensure_ascii=False)
        output_stream.write(json_str + '\n')

    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error during conversion: {e}", file=sys.stderr)
        sys.exit(1)


def detect_jsonl_from_content(input_stream: TextIO) -> tuple[bool, str]:
    """
    Detect if input is JSONL format by reading the first line.
    Returns (is_jsonl, first_line) tuple.
    """
    first_line = input_stream.readline()

    if not first_line.strip():
        return (False, first_line)

    # Try to parse the first line as JSON
    # If it parses successfully, it's JSONL; otherwise, it's a multi-line JSON
    try:
        json.loads(first_line.strip())
        return (True, first_line)
    except json.JSONDecodeError:
        return (False, first_line)


def main():
    """Main CLI entry point"""
    parser = argparse.ArgumentParser(
        prog='ddb_convert.py',
        description='Convert between DynamoDB JSON format and normal JSON format',
        formatter_class=argparse.RawDescriptionHelpFormatter
    )

    # Add mode as positional argument
    parser.add_argument('mode', choices=['from-ddb', 'to-ddb'],
                        help='Conversion mode: from-ddb or to-ddb')

    # Add optional arguments
    parser.add_argument('-i', '--input', dest='input_file',
                        help='Input file path (default: stdin)')
    parser.add_argument('-o', '--output', dest='output_file',
                        help='Output file path (default: stdout)')
    parser.add_argument('-p', '--pretty', action='store_true',
                        help='Pretty print output JSON')
    parser.add_argument('--without-item', action='store_true',
                        help='Omit top-level "Item" wrapper (only applies to to-ddb mode)')

    args = parser.parse_args()

    # Initialize converter
    converter = DynamoDBJSONConverter()

    # Open input stream with buffering
    if args.input_file:
        try:
            # Use buffered reader for better performance
            input_stream = io.BufferedReader(io.FileIO(args.input_file, 'r'))
            input_stream = io.TextIOWrapper(input_stream, encoding='utf-8')
        except FileNotFoundError:
            print(f"Error: File '{args.input_file}' not found", file=sys.stderr)
            sys.exit(1)
    else:
        input_stream = sys.stdin

    # Open output stream with buffering
    if args.output_file:
        try:
            # Use buffered writer for better performance
            output_stream = io.BufferedWriter(io.FileIO(args.output_file, 'w'))
            output_stream = io.TextIOWrapper(output_stream, encoding='utf-8')
        except IOError as e:
            print(f"Error: Cannot write to '{args.output_file}': {e}", file=sys.stderr)
            sys.exit(1)
    else:
        output_stream = sys.stdout

    try:
        # Detect if input is JSONL
        if args.input_file and args.input_file.endswith('.jsonl'):
            # Fast path: .jsonl extension means JSONL format
            is_jsonl = True
            first_line = ""
        else:
            # Detect from content by reading first line
            is_jsonl, first_line = detect_jsonl_from_content(input_stream)

        # Process based on detected format
        if is_jsonl:
            process_jsonl(input_stream, output_stream, converter, args.mode,
                         args.pretty, args.without_item, first_line)
        else:
            process_json(input_stream, output_stream, converter, args.mode,
                        args.pretty, args.without_item, first_line)
    finally:
        # Close files if they were opened
        if args.input_file:
            input_stream.close()
        if args.output_file:
            output_stream.close()


if __name__ == "__main__":
    main()
