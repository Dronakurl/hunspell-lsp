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

**Supported formats:**
- **Plain text**: `lang: en_US` (works anywhere in the document)
- `# lang: en_US` (shell-style)
- `// lang: en_US` (C-style)
- `<!-- lang: en_US -->` (HTML/Markdown)
- `; lang: en_US` (INI-style)
- `% lang: en_US` (LaTeX/TeX-style)

**Rules:**
- The first `lang:` pattern in the document is used
- Language can be specified in comments or plain text
- If no language is specified, it defaults to `en_US`

### Example

For a Python file:
```python
# lang: en_GB

def hello_world():
    print("Hello World")  # Any misspellings will be highlighted
```

### Code Actions

The LSP server provides automatic correction suggestions:

1. **Misspelled words are highlighted** with yellow warnings
2. **Hover over the word** to see Hunspell suggestions
3. **Apply quick fixes** to automatically replace misspelled words with suggestions
4. **Multiple suggestions** are provided when available

Example diagnostic message:
```
Possibly misspelled: dksadf. Suggestions: does, dad, sad, dads
```

For a JavaScript file:
```javascript
// lang: en_US

function hello() {
    console.log("Hello");  // Spell checking enabled
}
```

## Editor Configuration

### VS Code

Add to your `settings.json`:

```json
{
    "lsp.hunspell-lsp.command": "/path/to/hunspell-lsp",
    "lsp.hunspell-lsp.filetypes": ["text", "markdown", "python", "javascript"]
}
```

### Neovim

Using `nvim-lspconfig`:

```lua
require('lspconfig').hunspell_lsp.setup({
    cmd = {'/path/to/hunspell-lsp'},
    filetypes = {'text', 'markdown', 'python', 'javascript'},
})
```

### Emacs

Using `eglot`:

```elisp
(add-to-list 'eglot-server-programs
             '(text-mode . ("/path/to/hunspell-lsp")))
```

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

## License

This project is provided as-is for educational and practical use.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.