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

    # Create named pipe
    pipe_path = parent_dir / "example.pipe"
    if pipe_path.exists():
        pipe_path.unlink()
    os.mkfifo(str(pipe_path))

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
        status = "[0] 0:cat* 1:pv                                        \"cliqz\" " + time.strftime("%H:%M %d-%b-%y")
        gen.add_output(status.ljust(79), 0.01)
        gen.add_output("\x1b[m", 0.01)  # Reset colors

        # Left pane: show docker command already entered and running (cursor waiting)
        docker_cmd = "cat example.pipe | docker run --rm -i olpa/ddb_convert --pretty --unbuffered from-ddb"
        left_line = 1
        col = 1

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

        # Right pane: show pv command already entered and running
        pv_cmd = "pv -qL 20 example.json | tee example.pipe"
        right_line = 1
        col = 41

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

        # Start both processes
        print("Starting Docker process in left pane...")
        docker_process = subprocess.Popen(
            ["docker", "run", "--rm", "-i", "olpa/ddb_convert",
             "--pretty", "--unbuffered", "from-ddb"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(parent_dir)
        )

        print("Starting pv process in right pane...")
        pv_process = subprocess.Popen(
            ["pv", "-qL", "20", str(example_json)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(parent_dir)
        )

        # Read from pv and feed to docker while capturing outputs
        left_line = left_output_line
        right_line = right_output_line

        left_col = 1
        right_col = 41

        print("Processing data through pipe...")

        # Read pv output and display in both panes
        while True:
            chunk = pv_process.stdout.read(1)
            if not chunk:
                break

            # Send to docker
            docker_process.stdin.write(chunk)
            docker_process.stdin.flush()

            # Display in right pane
            char = chunk.decode('utf-8', errors='replace')
            if char == '\n':
                right_line += 1
                right_col = 41
                if right_line >= 22:
                    # Scroll right pane
                    right_line = 21
            else:
                if right_col < 79:
                    gen.add_output(f"\x1b[{right_line};{right_col}H{char}", 0.001)
                    right_col += 1

            # Check for docker output
            try:
                import select
                ready, _, _ = select.select([docker_process.stdout], [], [], 0)
                if ready:
                    output_chunk = docker_process.stdout.read(1)
                    if output_chunk:
                        out_char = output_chunk.decode('utf-8', errors='replace')
                        if out_char == '\n':
                            left_line += 1
                            left_col = 1
                            if left_line >= 22:
                                left_line = 21
                        else:
                            if left_col < 39:
                                gen.add_output(f"\x1b[{left_line};{left_col}H{out_char}", 0.001)
                                left_col += 1
            except:
                pass

        # Close docker stdin
        docker_process.stdin.close()

        # Get remaining docker output
        print("Collecting remaining Docker output...")
        remaining = docker_process.stdout.read().decode('utf-8', errors='replace')
        for char in remaining:
            if char == '\n':
                left_line += 1
                left_col = 1
                if left_line >= 22:
                    # Scroll
                    left_line = 21
            else:
                if left_col < 39:
                    gen.add_output(f"\x1b[{left_line};{left_col}H{char}", 0.001)
                    left_col += 1
                    if left_col >= 39:
                        left_line += 1
                        left_col = 1

        # Wait for processes to complete
        pv_process.wait()
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
        # Cleanup pipe
        if pipe_path.exists():
            pipe_path.unlink()

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
