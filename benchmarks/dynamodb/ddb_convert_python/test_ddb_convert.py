#!/usr/bin/env python3
"""
Tests for ddb_convert.py

Tests conversion between DynamoDB JSON format and normal JSON format
using fixtures from the ../fixture directory.
"""

import json
import subprocess
import sys
import os
from pathlib import Path
from typing import List, Tuple

# Path to the script
SCRIPT_PATH = Path(__file__).parent / "ddb_convert.py"
FIXTURE_DIR = Path(__file__).parent.parent / "fixture"


class TestColors:
    """ANSI color codes for test output"""
    GREEN = '\033[92m'
    RED = '\033[91m'
    YELLOW = '\033[93m'
    BLUE = '\033[94m'
    RESET = '\033[0m'
    BOLD = '\033[1m'


def run_conversion(mode: str, input_file: str, additional_args: List[str] = None) -> Tuple[str, str, int]:
    """
    Run ddb_convert.py with given parameters

    Returns: (stdout, stderr, returncode)
    """
    cmd = [sys.executable, str(SCRIPT_PATH), mode, "-i", input_file]
    if additional_args:
        cmd.extend(additional_args)

    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout, result.stderr, result.returncode


def compare_json_objects(obj1, obj2, path="root") -> List[str]:
    """
    Compare two JSON objects and return list of differences.
    Handles sets vs lists as equivalent if they contain the same elements.
    Handles DynamoDB L type vs SS/NS type as equivalent.
    """
    differences = []

    # Special handling for DynamoDB types: L vs SS/NS, S vs B
    if isinstance(obj1, dict) and isinstance(obj2, dict):
        # Check if obj1 has L and obj2 has SS
        if "L" in obj1 and len(obj1) == 1 and "SS" in obj2 and len(obj2) == 1:
            l_items = obj1["L"]
            ss_items = obj2["SS"]
            # Convert L format to SS format for comparison
            if all(isinstance(item, dict) and "S" in item and len(item) == 1 for item in l_items):
                l_as_ss = [item["S"] for item in l_items]
                # Compare as sets (order independent)
                if set(l_as_ss) != set(ss_items):
                    differences.append(f"{path}: L vs SS content mismatch")
                return differences

        # Check if obj1 has L and obj2 has NS
        if "L" in obj1 and len(obj1) == 1 and "NS" in obj2 and len(obj2) == 1:
            l_items = obj1["L"]
            ns_items = obj2["NS"]
            # Convert L format to NS format for comparison
            if all(isinstance(item, dict) and "N" in item and len(item) == 1 for item in l_items):
                l_as_ns = [item["N"] for item in l_items]
                # Compare as sets (order independent)
                if set(l_as_ns) != set(ns_items):
                    differences.append(f"{path}: L vs NS content mismatch")
                return differences

        # Check if obj1 has S (String) and obj2 has B (Binary) - treat as equivalent for base64
        if "S" in obj1 and len(obj1) == 1 and "B" in obj2 and len(obj2) == 1:
            # Both represent the same base64 string value
            if obj1["S"] == obj2["B"]:
                return differences  # They match
            else:
                differences.append(f"{path}: S vs B value mismatch - {obj1['S']} vs {obj2['B']}")
                return differences

        # Check if obj1 has B and obj2 has S - treat as equivalent for base64
        if "B" in obj1 and len(obj1) == 1 and "S" in obj2 and len(obj2) == 1:
            # Both represent the same base64 string value
            if obj1["B"] == obj2["S"]:
                return differences  # They match
            else:
                differences.append(f"{path}: B vs S value mismatch - {obj1['B']} vs {obj2['S']}")
                return differences

    # Special handling for set vs list comparison
    if isinstance(obj1, (list, set)) and isinstance(obj2, (list, set)):
        # Convert both to sets for comparison
        set1 = set(obj1) if not any(isinstance(x, (dict, list, set)) for x in obj1) else None
        set2 = set(obj2) if not any(isinstance(x, (dict, list, set)) for x in obj2) else None

        if set1 is not None and set2 is not None:
            # Both are simple sets/lists - compare as sets
            if set1 != set2:
                differences.append(f"{path}: Set/List content mismatch - {set1} vs {set2}")
            return differences
        else:
            # Contains complex objects, treat as lists
            list1 = list(obj1)
            list2 = list(obj2)
            if len(list1) != len(list2):
                differences.append(f"{path}: List/Set length mismatch - {len(list1)} vs {len(list2)}")
            else:
                for i, (item1, item2) in enumerate(zip(list1, list2)):
                    differences.extend(compare_json_objects(item1, item2, f"{path}[{i}]"))
            return differences

    if type(obj1) != type(obj2):
        differences.append(f"{path}: Type mismatch - {type(obj1).__name__} vs {type(obj2).__name__}")
        return differences

    if isinstance(obj1, dict):
        keys1 = set(obj1.keys())
        keys2 = set(obj2.keys())

        if keys1 != keys2:
            missing_in_2 = keys1 - keys2
            missing_in_1 = keys2 - keys1
            if missing_in_2:
                differences.append(f"{path}: Keys in first but not second: {missing_in_2}")
            if missing_in_1:
                differences.append(f"{path}: Keys in second but not first: {missing_in_1}")

        for key in keys1 & keys2:
            differences.extend(compare_json_objects(obj1[key], obj2[key], f"{path}.{key}"))

    elif isinstance(obj1, list):
        if len(obj1) != len(obj2):
            differences.append(f"{path}: List length mismatch - {len(obj1)} vs {len(obj2)}")
        else:
            for i, (item1, item2) in enumerate(zip(obj1, obj2)):
                differences.extend(compare_json_objects(item1, item2, f"{path}[{i}]"))

    elif isinstance(obj1, (int, float)) and isinstance(obj2, (int, float)):
        # Allow numeric comparison (int vs float)
        if abs(obj1 - obj2) > 1e-10:
            differences.append(f"{path}: Value mismatch - {obj1} vs {obj2}")

    elif obj1 != obj2:
        differences.append(f"{path}: Value mismatch - {obj1} vs {obj2}")

    return differences


def test_from_ddb_simple():
    """Test from-ddb conversion with simple_item fixture"""
    print(f"\n{TestColors.BLUE}Test: from-ddb with simple_item{TestColors.RESET}")

    input_file = FIXTURE_DIR / "simple_item_dynamodb.json"
    expected_file = FIXTURE_DIR / "simple_item_normal.json"

    stdout, stderr, returncode = run_conversion("from-ddb", str(input_file))

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse output and expected
    result = json.loads(stdout)
    with open(expected_file) as f:
        expected = json.load(f)

    differences = compare_json_objects(result, expected)

    if differences:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't match expected{TestColors.RESET}")
        for diff in differences:
            print(f"  {diff}")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_to_ddb_simple():
    """Test to-ddb conversion with simple_item fixture"""
    print(f"\n{TestColors.BLUE}Test: to-ddb with simple_item{TestColors.RESET}")

    input_file = FIXTURE_DIR / "simple_item_normal.json"
    expected_file = FIXTURE_DIR / "simple_item_dynamodb.json"

    stdout, stderr, returncode = run_conversion("to-ddb", str(input_file))

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse output - should have Item wrapper
    result = json.loads(stdout)

    # Load expected without Item wrapper and compare
    with open(expected_file) as f:
        expected_inner = json.load(f)

    # Result should have Item wrapper
    if "Item" not in result:
        print(f"{TestColors.RED}✗ FAILED: Output missing 'Item' wrapper{TestColors.RESET}")
        return False

    differences = compare_json_objects(result["Item"], expected_inner)

    if differences:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't match expected{TestColors.RESET}")
        for diff in differences:
            print(f"  {diff}")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_to_ddb_without_item():
    """Test to-ddb conversion with --without-item flag"""
    print(f"\n{TestColors.BLUE}Test: to-ddb with --without-item flag{TestColors.RESET}")

    input_file = FIXTURE_DIR / "simple_item_normal.json"
    expected_file = FIXTURE_DIR / "simple_item_dynamodb.json"

    stdout, stderr, returncode = run_conversion("to-ddb", str(input_file), ["--without-item"])

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse output - should NOT have Item wrapper
    result = json.loads(stdout)

    # Load expected
    with open(expected_file) as f:
        expected = json.load(f)

    # Result should NOT have Item wrapper
    if "Item" in result and len(result) == 1:
        print(f"{TestColors.RED}✗ FAILED: Output has 'Item' wrapper when it shouldn't{TestColors.RESET}")
        return False

    differences = compare_json_objects(result, expected)

    if differences:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't match expected{TestColors.RESET}")
        for diff in differences:
            print(f"  {diff}")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_jsonl_from_ddb():
    """Test from-ddb conversion with JSONL (yelp_business_mini)"""
    print(f"\n{TestColors.BLUE}Test: from-ddb with JSONL (yelp_business_mini){TestColors.RESET}")

    input_file = FIXTURE_DIR / "yelp_business_mini_dynamodb.jsonl"
    expected_file = FIXTURE_DIR / "yelp_business_mini_normal.jsonl"

    stdout, stderr, returncode = run_conversion("from-ddb", str(input_file))

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse JSONL output
    result_lines = [json.loads(line) for line in stdout.strip().split('\n') if line.strip()]

    # Load expected JSONL
    with open(expected_file) as f:
        expected_lines = [json.loads(line) for line in f if line.strip()]

    if len(result_lines) != len(expected_lines):
        print(f"{TestColors.RED}✗ FAILED: Number of lines mismatch - {len(result_lines)} vs {len(expected_lines)}{TestColors.RESET}")
        return False

    # Compare each line
    for i, (result_obj, expected_obj) in enumerate(zip(result_lines, expected_lines)):
        differences = compare_json_objects(result_obj, expected_obj, f"line {i+1}")
        if differences:
            print(f"{TestColors.RED}✗ FAILED: Line {i+1} doesn't match{TestColors.RESET}")
            for diff in differences[:5]:  # Show first 5 differences
                print(f"  {diff}")
            if len(differences) > 5:
                print(f"  ... and {len(differences) - 5} more differences")
            return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_jsonl_to_ddb():
    """Test to-ddb conversion with JSONL (yelp_business_mini)"""
    print(f"\n{TestColors.BLUE}Test: to-ddb with JSONL (yelp_business_mini){TestColors.RESET}")

    input_file = FIXTURE_DIR / "yelp_business_mini_normal.jsonl"
    expected_file = FIXTURE_DIR / "yelp_business_mini_dynamodb.jsonl"

    stdout, stderr, returncode = run_conversion("to-ddb", str(input_file))

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse JSONL output
    result_lines = [json.loads(line) for line in stdout.strip().split('\n') if line.strip()]

    # Load expected JSONL
    with open(expected_file) as f:
        expected_lines = [json.loads(line) for line in f if line.strip()]

    if len(result_lines) != len(expected_lines):
        print(f"{TestColors.RED}✗ FAILED: Number of lines mismatch - {len(result_lines)} vs {len(expected_lines)}{TestColors.RESET}")
        return False

    # Compare each line
    for i, (result_obj, expected_obj) in enumerate(zip(result_lines, expected_lines)):
        # Each result should have Item wrapper, each expected should too
        differences = compare_json_objects(result_obj, expected_obj, f"line {i+1}")
        if differences:
            print(f"{TestColors.RED}✗ FAILED: Line {i+1} doesn't match{TestColors.RESET}")
            for diff in differences[:5]:  # Show first 5 differences
                print(f"  {diff}")
            if len(differences) > 5:
                print(f"  ... and {len(differences) - 5} more differences")
            return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_complex_types_from_ddb():
    """Test from-ddb conversion with complex_types fixture"""
    print(f"\n{TestColors.BLUE}Test: from-ddb with complex_types{TestColors.RESET}")

    input_file = FIXTURE_DIR / "complex_types_dynamodb.json"
    expected_file = FIXTURE_DIR / "complex_types_normal.json"

    stdout, stderr, returncode = run_conversion("from-ddb", str(input_file))

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse output and expected
    result = json.loads(stdout)
    with open(expected_file) as f:
        expected = json.load(f)

    differences = compare_json_objects(result, expected)

    if differences:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't match expected{TestColors.RESET}")
        for diff in differences[:10]:  # Show first 10 differences
            print(f"  {diff}")
        if len(differences) > 10:
            print(f"  ... and {len(differences) - 10} more differences")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_complex_types_to_ddb():
    """Test to-ddb conversion with complex_types fixture"""
    print(f"\n{TestColors.BLUE}Test: to-ddb with complex_types{TestColors.RESET}")

    input_file = FIXTURE_DIR / "complex_types_normal.json"
    expected_file = FIXTURE_DIR / "complex_types_dynamodb.json"

    stdout, stderr, returncode = run_conversion("to-ddb", str(input_file), ["--without-item"])

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Parse output and expected
    result = json.loads(stdout)
    with open(expected_file) as f:
        expected = json.load(f)

    differences = compare_json_objects(result, expected)

    if differences:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't match expected{TestColors.RESET}")
        for diff in differences[:10]:  # Show first 10 differences
            print(f"  {diff}")
        if len(differences) > 10:
            print(f"  ... and {len(differences) - 10} more differences")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def test_pretty_output():
    """Test --pretty flag produces valid JSON"""
    print(f"\n{TestColors.BLUE}Test: --pretty flag{TestColors.RESET}")

    input_file = FIXTURE_DIR / "simple_item_normal.json"

    stdout, stderr, returncode = run_conversion("to-ddb", str(input_file), ["--pretty"])

    if returncode != 0:
        print(f"{TestColors.RED}✗ FAILED: Process returned error{TestColors.RESET}")
        print(f"stderr: {stderr}")
        return False

    # Check that output is valid JSON
    try:
        json.loads(stdout)
    except json.JSONDecodeError as e:
        print(f"{TestColors.RED}✗ FAILED: Output is not valid JSON: {e}{TestColors.RESET}")
        return False

    # Check that output contains indentation (pretty printed)
    if '  ' not in stdout:
        print(f"{TestColors.RED}✗ FAILED: Output doesn't appear to be pretty-printed{TestColors.RESET}")
        return False

    print(f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}")
    return True


def main():
    """Run all tests"""
    print(f"\n{TestColors.BOLD}Running ddb_convert.py tests{TestColors.RESET}")
    print("=" * 60)

    tests = [
        test_from_ddb_simple,
        test_to_ddb_simple,
        test_to_ddb_without_item,
        test_jsonl_from_ddb,
        test_jsonl_to_ddb,
        test_complex_types_from_ddb,
        test_complex_types_to_ddb,
        test_pretty_output,
    ]

    results = []
    for test_func in tests:
        try:
            passed = test_func()
            results.append((test_func.__name__, passed))
        except Exception as e:
            print(f"{TestColors.RED}✗ FAILED with exception: {e}{TestColors.RESET}")
            results.append((test_func.__name__, False))

    # Print summary
    print("\n" + "=" * 60)
    print(f"{TestColors.BOLD}Test Summary{TestColors.RESET}")
    print("=" * 60)

    passed_count = sum(1 for _, passed in results if passed)
    total_count = len(results)

    for test_name, passed in results:
        status = f"{TestColors.GREEN}✓ PASSED{TestColors.RESET}" if passed else f"{TestColors.RED}✗ FAILED{TestColors.RESET}"
        print(f"{test_name}: {status}")

    print("=" * 60)
    if passed_count == total_count:
        print(f"{TestColors.GREEN}{TestColors.BOLD}All {total_count} tests passed!{TestColors.RESET}")
        return 0
    else:
        print(f"{TestColors.RED}{TestColors.BOLD}{passed_count}/{total_count} tests passed{TestColors.RESET}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
