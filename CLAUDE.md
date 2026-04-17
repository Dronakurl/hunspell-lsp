# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Testing
```bash
# Build debug version
cargo build

# Build release version (faster, optimized)
cargo build --release

# Install to local cargo bin (for testing with Helix)
cargo install --path .

# Run all tests
cargo test

# Run only language extraction tests
cargo test extract_lang

# Run single test
cargo test test_extract_lang_with_hash_comment
```

### Dictionary Setup
The system requires Hunspell dictionaries in `/usr/share/hunspell/`. Each language needs both `.aff` and `.dic` files:
- `en_US.aff`, `en_US.dic` (American English)
- `de_DE.aff`, `de_DE.dic` (German)

## Architecture Overview

This is a Language Server Protocol (LSP) server that provides spell checking using Hunspell dictionaries. The architecture follows standard LSP patterns with custom spell-checking logic.

### Project Structure
- **`src/lib.rs`** - Core library with language extraction and dictionary loading
- **`src/main.rs`** - LSP server implementation with document state management
- **`tests/`** - Test markdown files for manual testing in Helix

### Key Components

**Language Extraction (`lib.rs`)**
- Single regex pattern matches multiple comment styles AND plain text
- Format: `lang: xx_YY` (works anywhere: comments, plain text, HTML comments)
- First match wins - searches entire document from start
- Supports: `#`, `//`, `;`, `%`, `<!-- -->`, and plain text

**Dictionary Loading (`lib.rs`)**
- Loads from `/usr/share/hunspell/{lang}/` directory
- Requires both `.aff` (affix) and `.dic` (dictionary) files
- Returns `None` if dictionary not found (graceful degradation)

**LSP Server (`main.rs`)**
- Communicates via stdin/stdout (standard LSP protocol)
- Maintains `DocumentState` per document with diagnostic data
- Smart code actions: cursor-position-aware suggestions
- Severity: HINT (not warning) for less intrusive editing

### Document State Management

The server tracks per-document state in `HashMap<String, DocumentState>`:
- `text`: Full document content
- `diagnostics`: HashMap keyed by diagnostic ID (`{uri}:{line}:{char}`)
- Each diagnostic contains `SpellCheckerData`: word, suggestions, range

### Smart Code Action Logic

**Cursor on misspelled word** → Show corrections for that word only
- Title format: `"--> '{suggestion}'"`
- Only one word's suggestions displayed

**Cursor elsewhere on line** → Show corrections for ALL misspelled words in that line
- Title format: `"'word' --> '{suggestion}'"`
- Multiple words' suggestions displayed
- Filters by `spell_data.range.start.line == cursor_line`

This prevents overwhelming the user when they just want to see available corrections for their current context.

### Diagnostic Publishing

When document changes:
1. Extract language using `extract_lang()` or default to `en_US`
2. Load appropriate Hunspell dictionary
3. Use regex `\b[\w']+\b` to find whole words only (prevents sub-word matches)
4. For each word: check spelling, generate suggestions via `dict.suggest()`
5. Store diagnostic data with exact character positions (not word indices)
6. Publish as HINT severity diagnostics

### Important Implementation Details

**Word Boundary Detection**
Uses regex `\b[\w']+\b` instead of simple whitespace splitting. This prevents false positives like detecting "ich" within "hinweisen" (German example).

**Range Storage**
Diagnostic ranges are stored with exact character positions:
```rust
range: Range {
    start: Position { line: line_idx as u32, character: word_start as u32 },
    end: Position { line: line_idx as u32, character: word_end as u32 }
}
```
This ensures code actions replace the exact word, not approximate positions.

**Language Specification Priority**
1. First `lang:` pattern in document wins
2. Searches entire document from beginning
3. Works in comments AND plain text
4. Case-insensitive matching

## Testing Strategy

### Manual Testing (tests/ directory)
Test files demonstrate various scenarios:
- `test_spell_check.md` - Basic spell checking
- `test_smart_code_actions.md` - Cursor position behavior
- `test_multiple_errors_same_line.md` - Multiple errors in one line
- `test_plain_text_lang.md` - Plain text language switching

### Unit Tests (src/lib.rs)
42 tests cover language extraction edge cases:
- Comment styles (HTML, shell, C, INI, LaTeX)
- Plain text matching
- Whitespace handling
- Multiple language specifications
- Invalid formats

## Target Editor

**Primary: Helix**
This LSP is primarily designed for Helix editor. Helix configuration in `~/.config/helix/languages.toml`:
```toml
[language-server.hunspell-lsp]
command = "/home/konrad/.cargo/bin/hunspell-lsp"

[[language]]
name = "markdown"
language-servers = ["hunspell-lsp"]
```

The smart code action behavior is optimized for Helix's code action system.