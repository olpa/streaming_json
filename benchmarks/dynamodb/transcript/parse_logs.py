#!/usr/bin/env python3
"""
Parse GNU time output from benchmark logs and convert to JSON format.
Each benchmark result becomes a two-line JSON object.
"""

import re
import sys


def simplify_filename(full_path):
    """Extract and simplify filename from path.

    yelp_academic_dataset_business.json -> business.json
    yelp_academic_dataset_checkin.json -> checkin.json
    """
    filename = full_path.split('/')[-1]
    if filename.startswith('yelp_academic_dataset_'):
        filename = filename.replace('yelp_academic_dataset_', '')
    return filename


def parse_time_line(line):
    """Parse GNU time output line.

    Example: 0.32user 0.13system 0:00.46elapsed 99%CPU (0avgtext+0avgdata 2780maxresident)k

    Returns dict with parsed fields.
    """
    result = {}

    # Parse user time
    m = re.search(r'(\d+\.\d+)user', line)
    if m:
        result['user'] = float(m.group(1))

    # Parse system time
    m = re.search(r'(\d+\.\d+)system', line)
    if m:
        result['system'] = float(m.group(1))

    # Parse elapsed time (format: M:SS.ss or H:MM:SS.ss)
    m = re.search(r'(\d+):(\d+\.\d+)elapsed', line)
    if m:
        minutes = int(m.group(1))
        seconds = float(m.group(2))
        result['elapsed'] = minutes * 60 + seconds

    # Parse CPU percentage
    m = re.search(r'(\d+)%CPU', line)
    if m:
        result['cpu_percent'] = int(m.group(1))

    # Parse maxresident memory (in KB)
    m = re.search(r'(\d+)maxresident', line)
    if m:
        result['maxresident_kb'] = int(m.group(1))

    return result


def parse_io_line(line):
    """Parse I/O and page fault line.

    Example: 0inputs+565880outputs (0major+125minor)pagefaults 0swaps

    Returns dict with parsed fields.
    """
    result = {}

    # Parse inputs
    m = re.search(r'(\d+)inputs', line)
    if m:
        result['inputs'] = int(m.group(1))

    # Parse outputs
    m = re.search(r'(\d+)outputs', line)
    if m:
        result['outputs'] = int(m.group(1))

    # Parse major page faults
    m = re.search(r'(\d+)major', line)
    if m:
        result['major_pagefaults'] = int(m.group(1))

    # Parse minor page faults
    m = re.search(r'(\d+)minor', line)
    if m:
        result['minor_pagefaults'] = int(m.group(1))

    # Parse swaps
    m = re.search(r'(\d+)swaps', line)
    if m:
        result['swaps'] = int(m.group(1))

    return result


def parse_command_line(line):
    """Parse time command line to extract mode and file.

    Example: time python ... to-ddb -i original-normal/yelp_academic_dataset_business.json -o ...

    Returns (mode, input_file)
    """
    mode = None
    input_file = None

    # Extract mode (to-ddb or from-ddb)
    # Check from-ddb first since it contains "to-ddb" as substring
    if 'from-ddb' in line:
        mode = 'from-ddb'
    elif 'to-ddb' in line:
        mode = 'to-ddb'
    else:
        raise ValueError(f"Mode not found in command line: {line}")

    # Extract input file after -i flag
    m = re.search(r'-i\s+(\S+)', line)
    if m:
        input_file = simplify_filename(m.group(1))
    else:
        raise ValueError(f"Input file not found in command line: {line}")

    return mode, input_file


def parse_log(log_file, tool_name):
    """Parse a log file and output JSON objects."""
    with open(log_file, 'r') as f:
        lines = [line.strip() for line in f if line.strip()]

    i = 0
    while i < len(lines):
        line = lines[i]

        # Look for "time" command lines
        if line.startswith('time '):
            mode, input_file = parse_command_line(line)

            # Next line should be the time output
            if i + 1 < len(lines):
                time_data = parse_time_line(lines[i + 1])

                # Line after that should be I/O data
                if i + 2 < len(lines):
                    io_data = parse_io_line(lines[i + 2])

                    # Combine all data
                    result = {
                        'id': {
                            'tool': tool_name,
                            'file': input_file,
                            'mode': mode
                        }
                    }
                    result.update(time_data)
                    result.update(io_data)

                    # Print as four-line JSON
                    print('{ "id": { "tool": "' + result['id']['tool'] + '", "file": "' + result['id']['file'] + '", "mode": "' + result['id']['mode'] + '" },')

                    # Line 2: user, system, elapsed, cpu_percent
                    line2_fields = []
                    for key in ['user', 'system', 'elapsed', 'cpu_percent']:
                        if key in result:
                            line2_fields.append(f'"{key}": {result[key]}')
                    print('    ' + ', '.join(line2_fields) + ',')

                    # Line 3: maxresident_kb, inputs, outputs
                    line3_fields = []
                    for key in ['maxresident_kb', 'inputs', 'outputs']:
                        if key in result:
                            line3_fields.append(f'"{key}": {result[key]}')
                    print('    ' + ', '.join(line3_fields) + ',')

                    # Line 4: major_pagefaults, minor_pagefaults, swaps
                    line4_fields = []
                    for key in ['major_pagefaults', 'minor_pagefaults', 'swaps']:
                        if key in result:
                            line4_fields.append(f'"{key}": {result[key]}')
                    print('    ' + ', '.join(line4_fields) + ' }')

                    i += 3
                    continue

        i += 1


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <log_file> <tool_name>", file=sys.stderr)
        sys.exit(1)

    log_file = sys.argv[1]
    tool_name = sys.argv[2]

    parse_log(log_file, tool_name)
