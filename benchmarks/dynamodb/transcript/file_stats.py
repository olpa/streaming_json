#!/usr/bin/env python3
"""
Generate JSON with file sizes.
Takes a list of JSON files and outputs their sizes in bytes.
"""

import os
import sys


def simplify_filename(filename):
    """Simplify filename by removing yelp_academic_dataset_ prefix.

    yelp_academic_dataset_business.json -> business.json
    other.json -> other.json
    """
    if filename.startswith('yelp_academic_dataset_'):
        return filename.replace('yelp_academic_dataset_', '')
    return filename


def get_file_sizes(file_list):
    """Get file sizes for a list of files."""
    results = []

    for filepath in file_list:
        if not os.path.exists(filepath):
            print(f"Warning: File not found: {filepath}", file=sys.stderr)
            continue

        size = os.path.getsize(filepath)
        filename = os.path.basename(filepath)
        simplified_name = simplify_filename(filename)

        results.append({
            "file": simplified_name,
            "size": size
        })

    return results


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <file1.json> <file2.json> ...", file=sys.stderr)
        sys.exit(1)

    file_list = sys.argv[1:]
    results = get_file_sizes(file_list)

    # Print JSON records
    for record in results:
        print('{ "file": "' + record['file'] + '", "size": ' + str(record['size']) + ' }')


if __name__ == '__main__':
    main()
