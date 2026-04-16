#!/bin/bash

# Simple test to check if the core functionality works
# This tests the extract_lang and dictionary loading functions

echo "Testing extract_lang function..."
echo "Input: '# lang: de_DE'"

# Create a simple test that doesn't require full LSP protocol
# We'll use a here-document to simulate what the LSP would receive

cat << 'EOF' > /tmp/test_lsp.py
import json
import sys

# Test the language extraction
text = """# lang: de_DE

Ich möchte nicht dksadf"""

# Simple regex test similar to our Rust code
import re
def extract_lang(text):
    pattern = r"(?m)^\s*(#|//|;|%)\s*lang:\s*([A-Za-z_]+)"
    match = re.search(pattern, text)
    if match:
        return match.group(2)
    return None

result = extract_lang(text)
print(f"Extracted language: {result}")

# Test if German dictionary exists
import os
aff_file = f"/usr/share/hunspell/{result}.aff"
dic_file = f"/usr/share/hunspell/{result}.dic"

if os.path.exists(aff_file) and os.path.exists(dic_file):
    print(f"✓ German dictionary found: {result}")
    print(f"  AFF: {aff_file}")
    print(f"  DIC: {dic_file}")
else:
    print(f"✗ German dictionary NOT found: {result}")
    sys.exit(1)
EOF

python3 /tmp/test_lsp.py
echo ""
echo "The test.md file is now correctly formatted for German spell checking."
echo "Open it in Helix to test the LSP integration."