#!/usr/bin/env python3
"""Test that LSP server responds to shutdown even during slow processing"""

import json
import subprocess
import sys
import time
import threading

def send_lsp_message(proc, message):
    """Send a message with proper LSP protocol headers"""
    content = json.dumps(message)
    content_bytes = content.encode('utf-8')
    header = f"Content-Length: {len(content_bytes)}\r\n\r\n"
    proc.stdin.write(header.encode('utf-8'))
    proc.stdin.write(content_bytes)
    proc.stdin.flush()

def read_lsp_message(proc, timeout=5):
    """Read a message with proper LSP protocol headers"""
    # Set timeout
    import select
    ready, _, _ = select.select([proc.stdout], [], [], timeout)
    if not ready:
        return None

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
        return None

    length = int(header_line.split(b':')[1].strip())
    proc.stdout.read(2)  # \r\n

    content = proc.stdout.read(length)
    return json.loads(content.decode('utf-8'))

def test_responsive_shutdown():
    """Test that server responds to shutdown requests quickly"""
    print("Starting LSP server...")
    proc = subprocess.Popen(
        ['./target/release/hunspell-lsp'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=False
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
            print("✓ Initialize successful")
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

        # Send multiple long documents rapidly
        print("Sending multiple long documents rapidly...")
        for i in range(5):
            typos = ["teh", "recieve", "seperate", "occured", "definately"]
            long_text = " ".join([typos[i % len(typos)] for i in range(5000)])

            didchange_request = {
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": f"file:///tmp/test{i}.md",
                        "languageId": "markdown",
                        "version": i + 1,
                    },
                    "contentChanges": [{
                        "text": long_text
                    }]
                }
            }
            send_lsp_message(proc, didchange_request)

        # Immediately send shutdown request (should interrupt processing)
        print("Sending immediate shutdown request...")
        shutdown_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": {}
        }
        start_time = time.time()
        send_lsp_message(proc, shutdown_request)

        # Wait for shutdown response
        shutdown_response = read_lsp_message(proc, timeout=10)
        elapsed = time.time() - start_time

        if shutdown_response and shutdown_response.get('id') == 2:
            print(f"✓ Got shutdown response in {elapsed:.2f}s (server remained responsive)")

            # Send exit notification
            exit_notif = {
                "jsonrpc": "2.0",
                "method": "exit",
                "params": {}
            }
            send_lsp_message(proc, exit_notif)

            time.sleep(1)
            if proc.poll() is not None:
                print("✓ Server exited cleanly")
                return True
            else:
                print("✗ Server did not exit")
                proc.terminate()
                return False
        else:
            print(f"✗ No shutdown response after {elapsed:.2f}s")
            proc.terminate()
            return False

    except Exception as e:
        print(f"✗ Error: {e}")
        proc.terminate()
        return False

if __name__ == "__main__":
    success = test_responsive_shutdown()
    sys.exit(0 if success else 1)