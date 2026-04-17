# hunspell-lsp

A Language Server Protocol (LSP) implementation that provides spell checking using Hunspell dictionaries.

## Features

- Real-time spell checking for text documents via LSP
- Support for multiple languages through Hunspell dictionaries
- Language specification via special comments in documents
- Full synchronization with text changes
- **Code Actions**: Automatic correction suggestions with quick-fix actions

## Building

### Prerequisites

- Rust toolchain (2024 edition or later)
- Hunspell library and development headers
- Hunspell dictionaries (typically installed in `/usr/share/hunspell/`)

### Install Dependencies

On Debian/Ubuntu:
```bash
sudo apt-get install build-essential libhunspell-dev hunspell
```

On Fedora/RHEL:
```bash
sudo dnf install hunspell-devel hunspell
```

On Arch Linux:
```bash
sudo pacman -S hunspell
```

### Build

```bash
cargo build
```

The compiled binary will be available at `target/debug/hunspell-lsp`.

## Usage

### As a Language Server

The LSP server communicates via stdin/stdout. To use it, you need to configure your editor to launch the binary:

```bash
/path/to/hunspell-lsp
```

### Language Specification

Specify the language for spell checking by adding a `lang:` pattern anywhere in your document:

**Format:** `lang: language_code`

**Examples:**
- `lang: en_US` (American English)
- `lang: de_DE` (German)
- `lang: fr_FR` (French)
- `lang: es_ES` (Spanish)

**How it works:**
- Place `lang: xx_YY` anywhere in your document (comments or plain text)
- The first `lang:` pattern found is used
- If no language is specified, defaults to `en_US`

**Example usage:**

In a Markdown file:
```markdown
<!-- lang: de_DE -->

Dies ist ein deutscher Text.

lang: en_US

This is English text.
```

In a Python file:
```python
# lang: en_GB

def hello_world():
    print("Hello World")
```

In plain text:
```text
lang: de_DE
Dieser Text wird auf Deutsch geprüft.
```

### Code Actions

The LSP server provides intelligent correction suggestions:

**Smart Code Action Behavior:**
- **Cursor on misspelled word** → Shows corrections for that word only
- **Cursor elsewhere on line** → Shows corrections for all misspelled words in the line
- **Severity** → Hints (not warnings) for less intrusive editing

**How it works:**
1. Misspelled words are highlighted with hints
2. Hover over the word to see Hunspell suggestions
3. Apply quick fixes to automatically replace misspelled words
4. Multiple suggestions provided when available

Example diagnostic message:
```
Possibly misspelled: dksadf. Suggestions: does, dad, sad, dads
```

**Code Action Examples:**
- On word: `"--> 'does'"` (shows only the suggestion)
- On line: `"'ihc' --> 'ich'"` (shows word and suggestion)

For a JavaScript file:
```javascript
// lang: en_US

function hello() {
    console.log("Hello");  // Spell checking enabled
}
```

## Editor Configuration

### Helix 

Add to your `~/.config/helix/languages.toml`:

```toml
[language-server.hunspell-lsp]
command = "/home/user/.cargo/bin/hunspell-lsp"

[[language]]
name = "markdown"
language-servers = ["hunspell-lsp"]

[[language]]
name = "text"
language-servers = ["hunspell-lsp"]
```

**Using smart code actions:**
- Move cursor to misspelled word → press code action key (typically `space` + `a` in Helix)
- Move cursor elsewhere on line → see all misspelled words in line

## Dictionary Location

The server looks for Hunspell dictionaries in `/usr/share/hunspell/` by default. Each language requires two files:
- `<lang>.aff` (affix file)
- `<lang>.dic` (dictionary file)

For example, for American English:
- `/usr/share/hunspell/en_US.aff`
- `/usr/share/hunspell/en_US.dic`

## How It Works

1. The LSP server receives document changes from the editor
2. Extracts the language specification from comments
3. Loads the appropriate Hunspell dictionary
4. Checks each word in the document
5. Publishes diagnostics (warnings) for misspelled words back to the editor

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
