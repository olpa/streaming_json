#!/usr/bin/env python3
"""
Visualize performance of different tools for JSON processing.

Creates two charts:
1. Time to process 1 GB of JSON (lower is better)
2. JSON bytes processed per second (higher is better)

Prints mean and standard deviation to stdout.
"""

import json
import sys
import numpy as np
import matplotlib.pyplot as plt


def load_json_objects(filename):
    """Generic loader for files containing multiple JSON objects.

    Uses JSONDecoder.raw_decode() to parse objects sequentially
    from the file content, handling any whitespace between objects.
    """
    objects = []

    with open(filename, 'r') as f:
        content = f.read()

    decoder = json.JSONDecoder()
    idx = 0

    while idx < len(content):
        # Skip whitespace
        while idx < len(content) and content[idx].isspace():
            idx += 1

        if idx >= len(content):
            break

        try:
            obj, offset = decoder.raw_decode(content[idx:])
            objects.append(obj)
            idx += offset
        except json.JSONDecodeError as e:
            print(f"Error parsing JSON in file '{filename}' at position {idx}: {e}", file=sys.stderr)
            print(f"Context: {content[idx:idx+100]}...", file=sys.stderr)
            raise

    return objects


def load_stats(stats_file):
    """Load benchmark statistics from file."""
    return load_json_objects(stats_file)


def load_file_sizes(file_sizes_file):
    """Load file sizes from file."""
    objects = load_json_objects(file_sizes_file)
    sizes = {}
    for record in objects:
        sizes[record['file']] = record['size']
    return sizes


def calculate_metrics(stats, file_sizes):
    """Calculate performance metrics per tool and mode.

    Only includes files >= 1 GB to reduce variance from small files.
    """
    tool_data = {}
    MIN_FILE_SIZE = 1024 ** 3  # 1 GB

    for record in stats:
        tool = record['id']['tool']
        mode = record['id']['mode']
        file_name = record['id']['file']
        cpu_time = record['user'] + record['system']

        if file_name not in file_sizes:
            continue

        file_size = file_sizes[file_name]

        # Skip files smaller than 1 GB
        if file_size < MIN_FILE_SIZE:
            continue

        # Calculate metrics
        gb_size = file_size / (1024 ** 3)
        time_per_gb = cpu_time / gb_size
        bytes_per_sec = file_size / cpu_time

        # Separate by tool and mode
        key = f"{tool} ({mode})"

        if key not in tool_data:
            tool_data[key] = {
                'time_per_gb': [],
                'bytes_per_sec': []
            }

        tool_data[key]['time_per_gb'].append(time_per_gb)
        tool_data[key]['bytes_per_sec'].append(bytes_per_sec)

    return tool_data


def print_statistics(tool_data):
    """Print mean and standard deviation for each tool."""
    print("Performance Statistics")
    print("=" * 80)
    print()

    for tool in sorted(tool_data.keys()):
        print(f"Tool: {tool}")
        print("-" * 40)

        time_data = np.array(tool_data[tool]['time_per_gb'])
        throughput_data = np.array(tool_data[tool]['bytes_per_sec'])

        print(f"  Time per GB (seconds):")
        print(f"    Mean: {np.mean(time_data):.2f} ± {np.std(time_data):.2f}")
        print(f"    Min:  {np.min(time_data):.2f}")
        print(f"    Max:  {np.max(time_data):.2f}")
        print()

        print(f"  Throughput (MB/s):")
        throughput_mb = throughput_data / (1024 ** 2)
        print(f"    Mean: {np.mean(throughput_mb):.2f} ± {np.std(throughput_mb):.2f}")
        print(f"    Min:  {np.min(throughput_mb):.2f}")
        print(f"    Max:  {np.max(throughput_mb):.2f}")
        print()


def create_visualizations(tool_data):
    """Create performance visualization charts."""
    # Map technical names to human-friendly names
    name_map = {
        'scan-json': 'scan_json',
        'rust': 'rust serde',
        'py-noboto': 'python no boto',
        'py-boto': 'python boto'
    }

    # Rename tools to human-friendly names
    renamed_data = {}
    for key, value in tool_data.items():
        # Parse "tool (mode)" format
        parts = key.split(' (')
        if len(parts) == 2:
            tool = parts[0]
            mode = parts[1].rstrip(')')
            friendly_name = name_map.get(tool, tool)
            new_key = f"{friendly_name} ({mode})"
        else:
            new_key = key
        renamed_data[new_key] = value

    tools = sorted(renamed_data.keys())

    # Calculate means for each metric
    time_means = [np.mean(renamed_data[tool]['time_per_gb']) for tool in tools]
    throughput_means = [np.mean(renamed_data[tool]['bytes_per_sec']) / (1024 ** 2) for tool in tools]

    # Create figure with two subplots
    # Size in inches: 640x480 pixels at 100 DPI = 6.4x4.8 inches
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(6.4, 4.8))

    # Chart 1: Time to process 1 GB (lower is better)
    bars1 = ax1.barh(tools, time_means, color='#e74c3c')  # Red for time/cost
    ax1.set_xlabel('Speed (s/GB)', fontsize=12)
    ax1.set_title('Time to Process 1 GB of JSON\n(Lower is Better)', fontsize=14, fontweight='bold')
    ax1.invert_xaxis()  # Invert so lower values are on the right (better)
    ax1.set_yticklabels([])  # Remove y-axis labels from left chart

    # Add value labels
    for i, (tool, val) in enumerate(zip(tools, time_means)):
        ax1.text(val, i, f' {val:.1f}s', va='center', ha='right', fontsize=9)

    # Chart 2: Throughput (higher is better)
    bars2 = ax2.barh(tools, throughput_means, color='#2ecc71')  # Green for throughput/performance
    ax2.set_xlabel('Throughput (MB/s)', fontsize=12)
    ax2.set_title('JSON Processing Throughput\n(Higher is Better)', fontsize=14, fontweight='bold')

    # Add value labels
    for i, (tool, val) in enumerate(zip(tools, throughput_means)):
        ax2.text(val, i, f' {val:.1f}', va='center', ha='left', fontsize=9)

    plt.tight_layout()

    # Save figure
    output_file = 'performance_comparison.png'
    plt.savefig(output_file, dpi=72, bbox_inches='tight')
    print(f"Visualization saved to: {output_file}")
    print()


def main():
    stats_file = 'stats.json'
    file_sizes_file = 'file_stats.json'

    # Load data
    stats = load_stats(stats_file)
    file_sizes = load_file_sizes(file_sizes_file)

    # Calculate metrics
    tool_data = calculate_metrics(stats, file_sizes)

    # Print statistics to stdout
    print_statistics(tool_data)

    # Create visualizations
    create_visualizations(tool_data)


if __name__ == '__main__':
    main()
