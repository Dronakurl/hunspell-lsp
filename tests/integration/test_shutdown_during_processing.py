#!/usr/bin/env python3
"""Test that LSP server remains responsive during slow processing"""

import json
import subprocess
import sys
import time
import threading
import select

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

def test_shutdown_during_long_processing():
    """Test that server responds to shutdown even while processing long document"""
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

        # Create a very long document with many typos
        print("Creating document with 50,000 words...")
        typos = ["teh", "recieve", "seperate", "occured", "definately"]
        long_text = " ".join([typos[i % len(typos)] for i in range(50000)])

        # Send didOpen notification in a thread
        def send_long_document():
            didopen_request = {
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": "file:///tmp/test.md",
                        "languageId": "markdown",
                        "version": 1,
                    },
                    "contentChanges": [{
                        "text": long_text
                    }]
                }
            }
            print("Sending long document...")
            send_lsp_message(proc, didopen_request)
            print("Document sent, processing should take a while...")

        # Start sending long document
        send_thread = threading.Thread(target=send_long_document)
        send_thread.start()

        # Wait immediately to send shutdown while processing is happening
        time.sleep(0.1)  # Very short delay to ensure document was sent

        # Try to send shutdown request WHILE processing
        print("Sending shutdown request during processing...")
        shutdown_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "shutdown",
            "params": {}
        }
        send_lsp_message(proc, shutdown_request)

        # Wait for shutdown response (should come quickly even if processing is slow)
        print("Waiting for shutdown response...")
        shutdown_response = read_lsp_message(proc, timeout=10)

        if shutdown_response and shutdown_response.get('id') == 2:
            print("✓ Got shutdown response quickly (server remained responsive)")

            # Send exit notification
            exit_notif = {
                "jsonrpc": "2.0",
                "method": "exit",
                "params": {}
            }
            send_lsp_message(proc, exit_notif)

            # Wait for process to exit
            send_thread.join(timeout=2)
            time.sleep(1)

            if proc.poll() is not None:
                print("✓ Server exited cleanly after shutdown")
                return True
            else:
                print("✗ Server did not exit")
                proc.terminate()
                return False
        else:
            print("✗ No shutdown response (server may have blocked)")
            proc.terminate()
            return False

    except Exception as e:
        print(f"✗ Error: {e}")
        proc.terminate()
        return False

if __name__ == "__main__":
    success = test_shutdown_during_long_processing()
    sys.exit(0 if success else 1)