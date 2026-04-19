# Harper Integration Test File

This file tests Harper integration with hunspell-lsp. Harper provides enhanced English spell checking and grammar checking.

## Expected Behavior

When Harper is installed and available:
- This file should be automatically routed to Harper for English checking
- Harper should detect spelling and grammar errors
- Diagnostics should appear in your editor for the issues below

## Test Cases

### 1. Spelling Errors
These should be flagged by Harper:

- This is a tpyo.
- Another mispelled word here.
- Definately use Harper for English.

### 2. Grammar Issues
These should be flagged by Harper:

- He don't like grammar errors.
- She have a cat.
- They was going to the store.

### 3. Style Suggestions
Harper may suggest style improvements:

- utilize this feature (suggest: use)
- prior to this event (suggest: before)
- in order to (suggest: to)

### 4. Common Writing Mistakes

- very unique (redundant)
- ATM machine (redundant acronym)
- free gift (redundant)

### 5. Wrong Words

- Their going to the park. (should be: They're)
- Its a beautiful day. (should be: It's)
- Your welcome! (should be: You're)

## Verification Steps

1. Make sure Harper is installed: `harper-ls --version`
2. Rebuild hunspell-lsp: `cargo build --release`
3. Open this file in Helix: `helix tests/test_harper_integration.md`
4. Check that diagnostics appear for the errors above
5. Note: Harper uses HINT severity by default

## Expected Diagnostic Messages

Harper should provide diagnostics like:
- "Did you mean to spell 'typo' this way?"
- "Did you mean 'doesn't'?"
- "Did you mean 'has'?"
- "You probably don't need to say 'prior' here; 'before' is simpler."

## Language Specification

lang: en_US

This ensures the file is treated as English and routed to Harper.
