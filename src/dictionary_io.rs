//! Async dictionary file I/O operations for user and workspace dictionaries.
//!
//! This module provides functions for reading, writing, and modifying dictionary files
//! in a plain text format (one word per line, sorted alphabetically).

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

/// Loads a dictionary file and returns a HashSet of words.
///
/// # Arguments
///
/// * `path` - Path to the dictionary file
///
/// # Returns
///
/// * `Ok(HashSet<String>)` - Set of words loaded from the file
/// * `Err` - If the file cannot be read or parsed
///
/// # Behavior
///
/// - Returns empty set if file doesn't exist (graceful degradation)
/// - Each line should contain one word
/// - Empty lines and whitespace-only lines are ignored
/// - Words are trimmed of leading/trailing whitespace
pub async fn load_dict(path: &Path) -> Result<HashSet<String>> {
    // If file doesn't exist, return empty set (not an error)
    if !path.exists() {
        return Ok(HashSet::new());
    }

    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read dictionary from {:?}", path))?;

    let mut words = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            words.insert(trimmed.to_string());
        }
    }

    Ok(words)
}

/// Saves a set of words to a dictionary file.
///
/// # Arguments
///
/// * `path` - Path to the dictionary file
/// * `words` - Set of words to save
///
/// # Returns
///
/// * `Ok(())` - If the dictionary was saved successfully
/// * `Err` - If the file cannot be written
///
/// # Behavior
///
/// - Creates parent directories if they don't exist
/// - Writes one word per line, sorted alphabetically
/// - Overwrites existing file if it exists
pub async fn save_dict(path: &Path, words: &HashSet<String>) -> Result<()> {
    // Ensure parent directories exist
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    // Sort words alphabetically for consistent output
    let mut sorted_words: Vec<&String> = words.iter().collect();
    sorted_words.sort();

    // Write to file
    let content = sorted_words.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join("\n");
    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("Failed to write dictionary to {:?}", path))?;

    Ok(())
}

/// Adds a single word to a dictionary file.
///
/// # Arguments
///
/// * `path` - Path to the dictionary file
/// * `word` - Word to add (will be trimmed)
///
/// # Returns
///
/// * `Ok(true)` - If word was added (wasn't already present)
/// * `Ok(false)` - If word was already present
/// * `Err` - If the operation failed
pub async fn add_word_to_dict(path: &Path, word: &str) -> Result<bool> {
    let trimmed_word = word.trim();

    if trimmed_word.is_empty() {
        return Ok(false);
    }

    // Load existing words
    let mut words = load_dict(path).await?;

    // Check if word already exists
    if words.contains(trimmed_word) {
        return Ok(false);
    }

    // Add the word and save
    words.insert(trimmed_word.to_string());
    save_dict(path, &words).await?;

    Ok(true)
}

/// Removes a single word from a dictionary file.
///
/// # Arguments
///
/// * `path` - Path to the dictionary file
/// * `word` - Word to remove (will be trimmed)
///
/// # Returns
///
/// * `Ok(true)` - If word was removed
/// * `Ok(false)` - If word wasn't present
/// * `Err` - If the operation failed
pub async fn remove_word_from_dict(path: &Path, word: &str) -> Result<bool> {
    let trimmed_word = word.trim();

    if trimmed_word.is_empty() {
        return Ok(false);
    }

    // Load existing words
    let mut words = load_dict(path).await?;

    // Remove the word if present
    let removed = words.remove(trimmed_word);

    if removed {
        save_dict(path, &words).await?;
    }

    Ok(removed)
}

/// Gets the default user dictionary path.
///
/// # Returns
///
/// * Path to user dictionary file (e.g., ~/.config/hunspell-lsp/user_dictionary.txt)
pub fn get_user_dict_path() -> std::path::PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));

    config_dir.join("hunspell-lsp").join("user_dictionary.txt")
}

/// Gets the workspace dictionary path for a given workspace directory.
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace/project directory
///
/// # Returns
///
/// * Path to workspace dictionary file (e.g., /workspace/.hunspell-dictionary.txt)
pub fn get_workspace_dict_path(workspace_path: &Path) -> std::path::PathBuf {
    workspace_path.join(".hunspell-dictionary.txt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_dict_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        // Create empty file
        tokio::fs::write(&dict_path, "").await.unwrap();

        let words = load_dict(&dict_path).await.unwrap();
        assert!(words.is_empty());
    }

    #[tokio::test]
    async fn test_load_dict_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("nonexistent.txt");

        // Should return empty set for nonexistent file
        let words = load_dict(&dict_path).await.unwrap();
        assert!(words.is_empty());
    }

    #[tokio::test]
    async fn test_load_dict_with_words() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        // Create file with words
        let content = "hello\nworld\nrust\n";
        tokio::fs::write(&dict_path, content).await.unwrap();

        let words = load_dict(&dict_path).await.unwrap();
        assert_eq!(words.len(), 3);
        assert!(words.contains("hello"));
        assert!(words.contains("world"));
        assert!(words.contains("rust"));
    }

    #[tokio::test]
    async fn test_save_and_load_dict() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        let mut words = HashSet::new();
        words.insert("apple".to_string());
        words.insert("banana".to_string());
        words.insert("cherry".to_string());

        save_dict(&dict_path, &words).await.unwrap();

        let loaded = load_dict(&dict_path).await.unwrap();
        assert_eq!(loaded, words);

        // Check that file is sorted
        let content = tokio::fs::read_to_string(&dict_path).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines, vec!["apple", "banana", "cherry"]);
    }

    #[tokio::test]
    async fn test_add_word_to_dict() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        // Add first word
        let added = add_word_to_dict(&dict_path, "test").await.unwrap();
        assert!(added);

        // Try to add same word again
        let added = add_word_to_dict(&dict_path, "test").await.unwrap();
        assert!(!added);

        // Add another word
        let added = add_word_to_dict(&dict_path, "another").await.unwrap();
        assert!(added);

        // Verify both words are in dict
        let words = load_dict(&dict_path).await.unwrap();
        assert_eq!(words.len(), 2);
        assert!(words.contains("test"));
        assert!(words.contains("another"));
    }

    #[tokio::test]
    async fn test_remove_word_from_dict() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        // Add words
        add_word_to_dict(&dict_path, "keep").await.unwrap();
        add_word_to_dict(&dict_path, "remove").await.unwrap();

        // Remove one word
        let removed = remove_word_from_dict(&dict_path, "remove").await.unwrap();
        assert!(removed);

        // Try to remove again
        let removed = remove_word_from_dict(&dict_path, "remove").await.unwrap();
        assert!(!removed);

        // Verify only one word remains
        let words = load_dict(&dict_path).await.unwrap();
        assert_eq!(words.len(), 1);
        assert!(words.contains("keep"));
        assert!(!words.contains("remove"));
    }

    #[tokio::test]
    async fn test_word_trimming() {
        let temp_dir = TempDir::new().unwrap();
        let dict_path = temp_dir.path().join("test_dict.txt");

        // Add word with spaces
        add_word_to_dict(&dict_path, "  test  ").await.unwrap();

        let words = load_dict(&dict_path).await.unwrap();
        assert_eq!(words.len(), 1);
        assert!(words.contains("test"));
    }
}