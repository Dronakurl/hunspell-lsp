#!/usr/bin/env python3
"""Test LSP server with proper protocol (Content-Length headers)"""

import json
import subprocess
import struct
import sys
import time

def send_lsp_message(proc, message):
    """Send a message with proper LSP protocol headers"""
    content = json.dumps(message)
    content_bytes = content.encode('utf-8')
    header = f"Content-Length: {len(content_bytes)}\r\n\r\n"

    print(f"Sending: {content[:100]}...")
    proc.stdin.write(header.encode('utf-8'))
    proc.stdin.write(content_bytes)
    proc.stdin.flush()

def read_lsp_message(proc):
    """Read a message with proper LSP protocol headers"""
    # Read Content-Length header
    header_line = b''
    while True:
        char = proc.stdout.read(1)
        if not char:
            return None
        header_line += char
        if char == b'\n':
            break

    if not header_line.startswith(b'Content-Length:'):
        print(f"WARNING: Unexpected header: {header_line}")
        return None

    length = int(header_line.split(b':')[1].strip())

    # Read the blank line
    proc.stdout.read(2)  # \r\n

    # Read the content
    content = proc.stdout.read(length)
    return json.loads(content.decode('utf-8'))

def test_lsp_with_long_text():
    print("Starting LSP server...")
    proc = subprocess.Popen(
        ['./target/release/hunspell-lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False  # Use bytes for proper protocol handling
    )

    try:
        time.sleep(0.1)

        # Initialize
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": 1234,
                "rootUri": None,
                "capabilities": {}
            }
        }

        send_lsp_message(proc, init_request)
        response = read_lsp_message(proc)
        if response:
            print(f"✓ Initialize successful")
        else:
            print("✗ No initialize response")
            return False

        # Send initialized notification
        initialized_notif = {
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }
        send_lsp_message(proc, initialized_notif)

        # Test with increasing document sizes
        test_sizes = [100, 1000, 5000, 10000, 20000]

        for size in test_sizes:
            print(f"\nTesting with {size} words...")

            # Create document with many actual typos (common misspellings)
            typos = ["teh", "recieve", "seperate", "occured", "definately", "wether", "thier", "goverment"]
            long_text = " ".join([typos[i % len(typos)] for i in range(size)])

            # Add some valid words too
            long_text += " " + " ".join(["the quick brown fox jumps over the lazy dog" for _ in range(size // 9)])

            # Send didOpen notification
            didopen_request = {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": "file:///tmp/test.md",
                        "languageId": "markdown",
                        "version": 1,
                        "text": long_text
                    }
                }
            }

            start_time = time.time()
            send_lsp_message(proc, didopen_request)

            # Try to read diagnostics
            try:
                # Set timeout using select
                import select
                ready, _, _ = select.select([proc.stdout], [], [], 30)  # 30 second timeout

                if ready:
                    diag_response = read_lsp_message(proc)
                    elapsed = time.time() - start_time

                    if diag_response and diag_response.get('method') == 'textDocument/publishDiagnostics':
                        diag_count = len(diag_response.get('params', {}).get('diagnostics', []))
                        print(f"✓ Got {diag_count} diagnostics in {elapsed:.2f}s")
                    else:
                        print(f"✗ Unexpected response: {diag_response.get('method') if diag_response else 'None'}")
                        return False
                else:
                    print(f"✗ Timeout after {time.time() - start_time:.2f}s")
                    return False

            except Exception as e:
                print(f"✗ Error: {e}")
                return False

        # Shutdown gracefully
        print("\nShutting down...")
        shutdown_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": {}
        }
        send_lsp_message(proc, shutdown_request)
        response = read_lsp_message(proc)

        exit_notif = {
            "jsonrpc": "2.0",
            "method": "exit",
            "params": {}
        }
        send_lsp_message(proc, exit_notif)

        time.sleep(1)
        if proc.poll() is not None:
            print(f"✓ Server exited cleanly")
            return True
        else:
            print("✗ Server did not exit")
            proc.terminate()
            return False

    except Exception as e:
        print(f"✗ Error: {e}")
        proc.terminate()
        return False

if __name__ == "__main__":
    success = test_lsp_with_long_text()
    sys.exit(0 if success else 1)