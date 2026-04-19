# Integration Tests

These tests simulate real LSP server communication to catch issues that unit tests miss.

## test_long_document.py

Tests performance with long documents containing many typos. This reproduces the original Helix crash issue where the server would timeout and cause Helix to crash.

**Run with:**
```bash
cd tests/integration
python3 test_long_document.py
```

**What it tests:**
- Proper LSP protocol communication (Content-Length headers)
- Long document handling (100 to 20,000 words)
- Server shutdown behavior
- Performance under load

## Performance Optimizations Applied

1. **Suggestion caching**: Same misspelled word gets cached suggestions
2. **Limited suggestions per word**: Max 10 suggestions instead of all
3. **Total diagnostic cap**: Max 1000 diagnostics per document
4. **Early termination**: Stops processing after reaching diagnostic limit

## Results

| Words | Before | After | Speedup |
|-------|--------|-------|---------|
| 100   | 1.01s  | 0.12s | 8.4x    |
| 1,000 | 10.09s | 0.22s | 46x     |
| 5,000 | TIMEOUT| 0.17s | ∞       |
| 20,000| N/A    | 0.17s | Instant |