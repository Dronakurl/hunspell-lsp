# Hunspell LSP Test Files

This directory contains test files for the Hunspell Language Server.

## Test Files

### `test_spell_check.md`
Basic German spell checking test with several misspelled words:
- `dksadf`
- `ihc` (should be "ich")
- `Kanl` (should be "Kann")
- `progammierung` (should be "Programmierung")
- `comuter` (should be "Computer")

### `test_smart_code_actions.md`
Demonstrates smart code action behavior:
- Cursor on misspelled word → shows corrections for that word only
- Cursor elsewhere on line → shows corrections for all misspelled words in line
- Tests multiple errors: "ihc", "Kanl", "progammierung", "comuter", "dksadf"

### `test_multiple_errors_same_line.md`
Tests multiple misspellings in a single line to verify cursor-position-based code actions.

### `test_plain_text_lang.md`
Demonstrates language specification in plain text (not just comments):
- Shows switching between German and English
- Tests that `lang:` pattern works anywhere in text

### `test_multiple_langs.md`
Tests that the first language specification is used when multiple exist.

### `test.md`
Original test file with HTML comment style language specification.

## Testing Code Actions

1. Open any test file in Helix
2. Move cursor to a misspelled word
3. Trigger code action (typically via keybinding)
4. Select a suggestion to replace the misspelled word

## Expected Behavior

- Misspelled words are highlighted with yellow warnings
- Hover shows word and available suggestions
- Code actions provide quick fixes
- Multiple misspellings in same line work independently
- Language can be specified via comments or plain text