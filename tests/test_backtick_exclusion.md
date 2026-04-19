# Test Backtick Exclusion

This file tests that terms in backticks are excluded from spell checking.

## Code Terms in Backticks

Use `HashMap` for storage. Create a `Vec` of items. Call `funcname` to execute.

The `variable` contains the result. Process the `datastruct` carefully.

## Misspelled Words in Backticks

These should be ignored:
- `teh` should be ignored
- Use `recieve` to get data
- Call `definately` for confirmation
- Process `seperate` items

## Regular Misspelled Words

These should be flagged:
- teh word is misspelled
- I recieve the package
- This is definately wrong
- Seperate the items

## Multi-word Backtick Phrases

- Check `misspelled word` examples
- Use `incorrect spelling` in code
- Test `typo detection` system

## Mixed Examples

The `funcname` handles teh data correctly. Use `HashMap` for storing teh results.

Call `teh_func` to process teh variables.