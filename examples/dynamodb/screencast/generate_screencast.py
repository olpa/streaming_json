#!/usr/bin/env python3
"""
Generate an asciinema screencast showing tmux with two vertical panes.
Left pane: Docker command reading from pipe
Right pane: pv command writing to pipe
"""

import json
import time
import subprocess
import tempfile
import os
import sys
import random
from pathlib import Path

class ScreencastGenerator:
    def __init__(self, width=80, height=24):
        self.width = width
        self.height = height
        self.events = []
        self.start_time = time.time()

    def add_output(self, text, delay=0.0):
        """Add output event with optional delay from previous event"""
        if delay > 0:
            time.sleep(delay)
        elapsed = time.time() - self.start_time
        self.events.append([elapsed, "o", text])

    def type_text(self, text, min_delay=0.05, max_delay=0.15):
        """Simulate typing text character by character"""
        for char in text:
            delay = random.uniform(min_delay, max_delay)
            self.add_output(char, delay)

    def send_key(self, key_sequence, delay=0.1):
        """Send special key sequences (Enter, etc.)"""
        self.add_output(key_sequence, delay)

    def wait(self, duration):
        """Wait for specified duration"""
        time.sleep(duration)

    def save(self, filename):
        """Save screencast to file"""
        header = {
            "version": 2,
            "width": self.width,
            "height": self.height,
            "timestamp": int(time.time()),
            "env": {
                "SHELL": "/bin/bash",
                "TERM": "xterm-256color"
            }
        }

        with open(filename, 'w') as f:
            f.write(json.dumps(header) + '\n')
            for event in self.events:
                f.write(json.dumps(event) + '\n')

        print(f"Screencast saved to: {filename}")

def run_tmux_session():
    """Run the actual tmux session and capture output"""

    # Get the directory where the script is located
    script_dir = Path(__file__).parent.absolute()
    parent_dir = script_dir.parent

    # Verify files exist
    example_json = parent_dir / "example.json"
    if not example_json.exists():
        print(f"Error: {example_json} not found")
        sys.exit(1)

    try:
        gen = ScreencastGenerator(width=80, height=24)

        # Start with tmux already open and panes split
        # Tmux initialization sequence
        gen.add_output("\x1b[?1049h", 0.02)  # Alternative screen buffer
        gen.add_output("\x1b[?1h\x1b=", 0.01)  # Application keypad mode
        gen.add_output("\x1b[H\x1b[2J", 0.01)  # Clear screen
        gen.add_output("\x1b[?25h", 0.01)  # Show cursor

        # Draw tmux layout with vertical split
        gen.wait(0.05)

        # Clear screen and draw panes
        for i in range(1, 23):
            gen.add_output(f"\x1b[{i};1H\x1b[K", 0.001)  # Clear line

        # Draw vertical separator at column 40
        for i in range(1, 23):
            gen.add_output(f"\x1b[{i};40H│", 0.001)

        # Draw status line
        gen.add_output("\x1b[23;1H\x1b[30m\x1b[42m", 0.01)
        status = "[0] 0:ddb_convert* 1:pv                                        \"ddb-demo\" Jan-26"
        gen.add_output(status.ljust(79), 0.01)
        gen.add_output("\x1b[m", 0.01)  # Reset colors

        # Left pane: show banner and docker command already entered and running (cursor waiting)
        banner1_text = "#\n# Processing JSON stream here\n#\n\n"

        left_line = 1
        col = 1

        # Display banner1
        for char in banner1_text:
            if char == '\n':
                left_line += 1
                col = 1
            elif col < 40:
                gen.add_output(f"\x1b[{left_line};{col}H{char}")
                col += 1
                if col >= 40:
                    left_line += 1
                    col = 1

        # Add command with $ prefix
        docker_cmd = "$ cat example.pipe | docker run --rm -i olpa/ddb_convert --pretty --unbuffered from-ddb"

        # Display command with line wrapping
        for char in docker_cmd:
            if col > 39:
                left_line += 1
                col = 1
            gen.add_output(f"\x1b[{left_line};{col}H{char}", 0.001)
            col += 1

        # Position cursor at start of next line in left pane (where output will appear)
        left_output_line = left_line + 1
        gen.add_output(f"\x1b[{left_output_line};1H", 0.01)

        # Right pane: show banner and pv command already entered and running
        banner2_text = "#\n# Sending JSON\n#\n\n"

        right_line = 1
        col = 41

        # Display banner2
        for char in banner2_text:
            if char == '\n':
                right_line += 1
                col = 41
            elif col <= 79:
                gen.add_output(f"\x1b[{right_line};{col}H{char}")
                col += 1
                if col > 79:
                    right_line += 1
                    col = 41

        # Add command with $ prefix
        pv_cmd = "$ pv -qL 20 example.json | tee example.pipe"

        for char in pv_cmd:
            if col > 79:
                right_line += 1
                col = 41
            gen.add_output(f"\x1b[{right_line};{col}H{char}", 0.001)
            col += 1

        # Position cursor at start of next line in right pane (where output will appear)
        right_output_line = right_line + 1
        gen.add_output(f"\x1b[{right_output_line};41H", 0.01)

        gen.wait(0.3)

        # Start Docker process
        print("Starting Docker process in left pane...")
        docker_process = subprocess.Popen(
            ["./target/release/ddb_convert",
             "--pretty", "--unbuffered", "from-ddb"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,  # Unbuffered
            cwd=str(parent_dir)
        )

        # Read the JSON file content
        print(f"Reading {example_json}...")
        with open(example_json, 'r') as f:
            json_content = f.read()

        # Setup for feeding data at 20 chars/second
        chars_per_second = 20
        delay_per_char = 1.0 / chars_per_second

        left_line = left_output_line
        right_line = right_output_line
        left_col = 1
        right_col = 41

        print(f"Feeding data at {chars_per_second} chars/second (unbuffered)...")

        # Feed data character by character at 20 chars/second
        import select
        import fcntl

        # Set docker stdout to non-blocking
        flags = fcntl.fcntl(docker_process.stdout, fcntl.F_GETFL)
        fcntl.fcntl(docker_process.stdout, fcntl.F_SETFL, flags | os.O_NONBLOCK)

        for char in json_content:
            # Send to docker (unbuffered)
            docker_process.stdin.write(char.encode('utf-8'))
            docker_process.stdin.flush()

            # Display in right pane
            if char == '\n':
                right_line += 1
                right_col = 41
                if right_line >= 22:
                    # Scroll right pane
                    right_line = 21
            else:
                if right_col <= 79:
                    gen.add_output(f"\x1b[{right_line};{right_col}H{char}", 0.001)
                    right_col += 1

            # Check for docker output (non-blocking)
            try:
                ready, _, _ = select.select([docker_process.stdout], [], [], 0)
                if ready:
                    output_data = docker_process.stdout.read()
                    if output_data:
                        output_str = output_data.decode('utf-8', errors='replace')
                        for out_char in output_str:
                            if out_char == '\n':
                                left_line += 1
                                left_col = 1
                                if left_line >= 22:
                                    left_line = 21
                            else:
                                if left_col < 40:
                                    gen.add_output(f"\x1b[{left_line};{left_col}H{out_char}", 0.001)
                                    left_col += 1
            except:
                pass

            # Delay to achieve 20 chars/second
            time.sleep(delay_per_char)

        # Close docker stdin
        docker_process.stdin.close()

        # Get remaining docker output
        print("Collecting remaining Docker output...")
        time.sleep(0.5)  # Give docker time to finish processing

        # Set back to blocking for final read
        fcntl.fcntl(docker_process.stdout, fcntl.F_SETFL, flags)

        remaining = docker_process.stdout.read().decode('utf-8', errors='replace')
        for char in remaining:
            if char == '\n':
                left_line += 1
                left_col = 1
                if left_line >= 22:
                    # Scroll
                    left_line = 21
            else:
                if left_col < 40:
                    gen.add_output(f"\x1b[{left_line};{left_col}H{char}", 0.001)
                    left_col += 1
                    if left_col >= 40:
                        left_line += 1
                        left_col = 1

        # Wait for process to complete
        docker_process.wait()

        gen.wait(1.0)

        # Exit tmux
        gen.add_output("\x1b[?1049l", 0.1)  # Exit alternative screen

        gen.wait(0.5)

        # Save screencast
        output_file = script_dir / "generated.cast"
        gen.save(str(output_file))

        return 0

    finally:
        pass  # No pipe cleanup needed

if __name__ == "__main__":
    try:
        sys.exit(run_tmux_session())
    except KeyboardInterrupt:
        print("\nInterrupted")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)
