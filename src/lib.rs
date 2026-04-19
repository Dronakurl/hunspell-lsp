use hunspell_rs::Hunspell;
use regex::Regex;

/// Checks if a word should be ignored during spell checking based on heuristics.
///
/// This function provides an extensible system for avoiding false positives
/// by identifying words that match certain patterns or criteria.
///
/// # Arguments
///
/// * `word` - The word to check
/// * `line_context` - The full line containing the word (for context-aware heuristics)
///
/// # Returns
///
/// * `true` - If the word should be ignored (not spell checked)
/// * `false` - If the word should be spell checked normally
///
/// # Examples
///
/// ```
/// use hunspell_lsp::should_ignore_word;
///
/// // Language codes in lang: patterns are ignored
/// assert!(should_ignore_word("en_US", "lang: en_US"));
/// assert!(should_ignore_word("de_DE", "lang: de_DE"));
///
/// // Regular words are not ignored
/// assert!(!should_ignore_word("hello", "hello world"));
/// ```
pub fn should_ignore_word(word: &str, line_context: &str) -> bool {
    // Define heuristic patterns that should be ignored
    let ignore_heuristics: Vec<Box<dyn Fn(&str, &str) -> bool>> = vec![
        // 1. Language codes in lang: specifications (e.g., "en_US", "de_DE", "fr_FR")
        // Only ignore language codes that appear to be in a specification context
        Box::new(|word: &str, context: &str| -> bool {
            // Match language codes like: xx_YY, xx-YE, de_DE, en_US, zh-CN, etc.
            let lang_code_pattern = Regex::new(r"^[a-z]{2}[_-][A-Z]{2,3}$").unwrap();
            if !lang_code_pattern.is_match(word) {
                return false;
            }

            // Only ignore if context suggests it's a language specification
            // Check if the line looks like a language specification
            let lang_spec_pattern = Regex::new(r"(?:lang|language):\s*([a-zA-Z_\-]+)|<!--\s*lang:\s*([a-zA-Z_\-]+)|#\s*lang:\s*([a-zA-Z_\-]+)").unwrap();
            if let Some(caps) = lang_spec_pattern.captures(context) {
                // Check if any captured group matches our word
                for i in 1..4 {
                    if let Some(matched) = caps.get(i) {
                        if matched.as_str() == word {
                            return true;
                        }
                    }
                }
            }

            false
        }),

        // 2. Words in lang: specification lines (context-aware)
        Box::new(|word: &str, context: &str| -> bool {
            // Only ignore if the line is primarily a language specification
            // Pattern: "lang: xx_YY" or "language: xx_YY" (not embedded in sentences)
            let lang_spec_pattern = Regex::new(r"^\s*(?:lang|language):\s*([a-zA-Z_\-]+)\s*(?:-->|$|\n)").unwrap();
            if let Some(caps) = lang_spec_pattern.captures(context) {
                if let Some(lang_code) = caps.get(1) {
                    if word == lang_code.as_str() {
                        return true;
                    }
                }
            }
            false
        }),

        // 3. File extensions and technical identifiers
        Box::new(|word: &str, _context: &str| -> bool {
            // File extensions like .md, .rs, .toml, etc.
            if word.starts_with('.') && word.len() <= 10 {
                return true;
            }
            // Common technical patterns
            let tech_patterns = vec![
                r"^[a-z_]+_[a-z_]+$",  // snake_case variables (likely)
                r"^[A-Z_]+$",           // ALL_CAPS constants
            ];
            tech_patterns.iter().any(|pattern| {
                Regex::new(pattern).unwrap().is_match(word)
            })
        }),

        // 4. Words in backticks (code/technical terms)
        Box::new(|word: &str, context: &str| -> bool {
            // Find all backtick-enclosed segments in the line
            let backtick_pattern = Regex::new(r"`([^`]+)`").unwrap();
            for caps in backtick_pattern.captures_iter(context) {
                if let Some(matched) = caps.get(1) {
                    let backtick_content = matched.as_str();
                    // Split by whitespace and check for exact word matches
                    for backtick_word in backtick_content.split_whitespace() {
                        // Remove any punctuation from the backtick word for comparison
                        let clean_backtick_word = backtick_word.trim_matches(|c: char| !c.is_alphabetic());
                        if clean_backtick_word == word {
                            return true;
                        }
                    }
                }
            }
            false
        }),

        // 5. Single letter words that might be initials or abbreviations
        Box::new(|word: &str, _context: &str| -> bool {
            word.len() == 1 && word.chars().next().map_or(false, |c| c.is_alphabetic())
        }),
    ];

    // Check each heuristic
    for heuristic in ignore_heuristics {
        if heuristic(word, line_context) {
            return true;
        }
    }

    false
}

/// Extracts language specification from text comments or plain text.
///
/// Supports multiple formats:
/// - HTML comments for Markdown: <!-- lang: xx_YY -->
/// - Shell-style: # lang: xx_YY
/// - C-style: // lang: xx_YY
/// - INI-style: ; lang: xx_YY
/// - LaTeX/TeX-style: % lang: xx_YY
/// - Plain text anywhere: lang: xx_YY
///
/// Format: "lang: xx_YY" where xx is language code and YY is country code
///
/// Note: If multiple lang: specifications exist, the first one in the document is used.
///
/// # Examples
///
/// ```
/// use hunspell_lsp::extract_lang;
///
/// assert_eq!(extract_lang("<!-- lang: en_US -->"), Some("en_US".to_string()));
/// assert_eq!(extract_lang("# lang: de_DE"), Some("de_DE".to_string()));
/// assert_eq!(extract_lang("// lang: de_DE"), Some("de_DE".to_string()));
/// assert_eq!(extract_lang("; lang: fr_FR"), Some("fr_FR".to_string()));
/// assert_eq!(extract_lang("% lang: es_ES"), Some("es_ES".to_string()));
/// assert_eq!(extract_lang("Some text lang: en_US here"), Some("en_US".to_string()));
/// assert_eq!(extract_lang("No lang here"), None);
/// ```
pub fn extract_lang(text: &str) -> Option<String> {
    // Combined regex that matches all comment styles and plain text
    // This ensures we find the FIRST lang: specification regardless of format
    let re = Regex::new(r"(?mi)(<!--\s*lang:\s*([A-Za-z_]+)\s*-->|^\s*(#|//|;|%)\s*lang:\s*([A-Za-z_]+)|lang:\s*([A-Za-z_]+))").unwrap();

    if let Some(caps) = re.captures(text) {
        // The regex has multiple groups: HTML comments (group 2), other comments (group 4), plain text (group 5)
        if let Some(html_lang) = caps.get(2) {
            return Some(html_lang.as_str().to_string());
        } else if let Some(other_lang) = caps.get(4) {
            return Some(other_lang.as_str().to_string());
        } else if let Some(plain_lang) = caps.get(5) {
            return Some(plain_lang.as_str().to_string());
        }
    }

    None
}

/// Loads a Hunspell dictionary for the specified language.
///
/// Looks for dictionary files in `/usr/share/hunspell/<lang>/`
/// Requires both .aff and .dic files to be present.
///
/// # Arguments
///
/// * `lang` - Language code (e.g., "en_US", "de_DE")
///
/// # Returns
///
/// * `Some(Hunspell)` - If dictionary files are found and loaded successfully
/// * `None` - If dictionary files are not found or cannot be loaded
pub fn load_dict(lang: &str) -> Option<Hunspell> {
    let base = format!("/usr/share/hunspell/{}", lang);
    let aff = format!("{}.aff", base);
    let dic = format!("{}.dic", base);

    if !std::path::Path::new(&aff).exists() || !std::path::Path::new(&dic).exists() {
        return None;
    }

    // Try to load the dictionary, catching any panics from encoding issues
    std::panic::catch_unwind(|| {
        Hunspell::new(&aff, &dic)
    }).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_language_codes() {
        // Test language code pattern matching
        assert!(should_ignore_word("en_US", "lang: en_US"));
        assert!(should_ignore_word("de_DE", "lang: de_DE"));
        assert!(should_ignore_word("fr_FR", "lang: fr_FR"));
        assert!(should_ignore_word("zh_CN", "lang: zh_CN"));
        assert!(should_ignore_word("es_ES", "lang: es_ES"));

        // Test with different separators
        assert!(should_ignore_word("en-US", "lang: en-US"));
        assert!(should_ignore_word("de-DE", "lang: de-DE"));

        // Test that regular words are not ignored
        assert!(!should_ignore_word("hello", "hello world"));
        assert!(!should_ignore_word("world", "hello world"));
        assert!(!should_ignore_word("testing", "testing code"));
    }

    #[test]
    fn test_should_ignore_context_aware() {
        // Test context-aware language specification (must be proper lang: format)
        assert!(should_ignore_word("en_US", "lang: en_US"));
        assert!(should_ignore_word("de_DE", "language: de_DE"));
        assert!(should_ignore_word("fr_FR", "<!-- lang: fr_FR -->"));
        assert!(should_ignore_word("en_US", "lang: en_US"));

        // Test that language codes in regular text are NOT ignored
        assert!(!should_ignore_word("en_US", "I like en_US culture"));
        assert!(!should_ignore_word("de_DE", "The de_DE region"));

        // Test that lang: specs are recognized even in longer lines
        assert!(should_ignore_word("en_US", "This is lang: en_US in a sentence"));
        assert!(should_ignore_word("es_ES", "We use lang: es_ES for Spanish"));

        // Test that language codes without lang: prefix are NOT ignored
        assert!(!should_ignore_word("es_ES", "We support es_ES and de_DE"));
    }

    #[test]
    fn test_should_ignore_technical_patterns() {
        // Test file extensions
        assert!(should_ignore_word(".md", "README.md"));
        assert!(should_ignore_word(".rs", "main.rs"));
        assert!(should_ignore_word(".toml", "Cargo.toml"));

        // Test single letters
        assert!(should_ignore_word("a", "a variable"));
        assert!(should_ignore_word("x", "x coordinate"));

        // Test that normal multi-letter words are not ignored
        assert!(!should_ignore_word("var", "var variable"));
        assert!(!should_ignore_word("file", "file.txt"));
    }

    #[test]
    fn test_should_ignore_real_words() {
        // Ensure real words are not ignored
        assert!(!should_ignore_word("hello", "hello world"));
        assert!(!should_ignore_word("world", "hello world"));
        assert!(!should_ignore_word("rust", "rust programming"));
        assert!(!should_ignore_word("spell", "spell checker"));
    }

    #[test]
    fn test_should_ignore_backtick_terms() {
        // Test that words in backticks are ignored
        assert!(should_ignore_word("funcname", "use `funcname` to call"));
        assert!(should_ignore_word("variable", "the `variable` contains"));
        assert!(should_ignore_word("teh", "misspelled `teh` in backticks"));
        assert!(should_ignore_word("recieve", "typo `recieve` should be ignored"));

        // Test technical terms in backticks
        assert!(should_ignore_word("HashMap", "Use `HashMap` for storage"));
        assert!(should_ignore_word("Vec", "Create a `Vec` of items"));

        // Test that same words outside backticks are NOT ignored
        assert!(!should_ignore_word("funcname", "use funcname to call"));
        assert!(!should_ignore_word("variable", "the variable contains"));
        assert!(!should_ignore_word("HashMap", "Use HashMap for storage"));
    }

    #[test]
    fn test_should_ignore_multiple_backticks() {
        // Test multiple backticked terms in one line
        assert!(should_ignore_word("first", "`first` and `second` terms"));
        assert!(should_ignore_word("second", "`first` and `second` terms"));

        // Test that words outside backticks are not ignored in mixed line
        assert!(!should_ignore_word("and", "`first` and `second` terms"));
        assert!(!should_ignore_word("terms", "`first` and `second` terms"));
    }

    #[test]
    fn test_should_ignore_backticks_with_spaces() {
        // Test backticks with spaces inside
        assert!(should_ignore_word("word", "`multi word phrase` here"));
        assert!(should_ignore_word("multi", "`multi word phrase` here"));
        assert!(should_ignore_word("phrase", "`multi word phrase` here"));

        // Test normal words not in backticks
        assert!(!should_ignore_word("here", "`multi word phrase` here"));
        assert!(!should_ignore_word("test", "`multi word phrase` test"));

        // Test exact match behavior
        assert!(should_ignore_word("misspelled", "check `misspelled word` here"));
        assert!(should_ignore_word("word", "check `misspelled word` here"));
        assert!(!should_ignore_word("check", "check `misspelled word` here"));
        assert!(!should_ignore_word("here", "check `misspelled word` here"));
    }

    #[test]
    fn test_extract_lang_with_html_comment() {
        let text = "<!-- lang: en_US -->\nSome content here";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_with_html_comment_case_insensitive() {
        let text = "<!-- LANG: EN_US -->\nSome content here";
        assert_eq!(extract_lang(text), Some("EN_US".to_string()));
    }

    #[test]
    fn test_extract_lang_with_html_comment_with_spaces() {
        let text = "<!--  lang:  de_DE  -->\nSome content here";
        assert_eq!(extract_lang(text), Some("de_DE".to_string()));
    }

    #[test]
    fn test_extract_lang_with_hash_comment() {
        let text = "# lang: en_US\nSome content here";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_html_comment_priority_over_hash() {
        let text = "<!-- lang: en_US -->\n# lang: de_DE\nSome content";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_markdown_heading_not_recognized() {
        // In Markdown, # at start of line is a heading, not a comment
        // So "# lang: en_US" in a heading should NOT be recognized as language spec
        let text = "# lang: en_US\nThis is a heading, not a comment";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_hash_comment_with_leading_space() {
        // When # is preceded by spaces, it's more likely to be a comment
        let text = " # lang: en_US\nSome content here";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_with_double_slash_comment() {
        let text = "// lang: de_DE\nSome content here";
        assert_eq!(extract_lang(text), Some("de_DE".to_string()));
    }

    #[test]
    fn test_extract_lang_with_semicolon_comment() {
        let text = "; lang: fr_FR\nSome content here";
        assert_eq!(extract_lang(text), Some("fr_FR".to_string()));
    }

    #[test]
    fn test_extract_lang_with_percent_comment() {
        let text = "% lang: es_ES\nSome content here";
        assert_eq!(extract_lang(text), Some("es_ES".to_string()));
    }

    #[test]
    fn test_extract_lang_with_whitespace() {
        let text = "  # lang: en_GB  \nSome content here";
        assert_eq!(extract_lang(text), Some("en_GB".to_string()));
    }

    #[test]
    fn test_extract_lang_not_at_start() {
        let text = "Some content\n# lang: en_US\nMore content";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_no_lang_specified() {
        let text = "# This is just a comment\nSome content";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_extract_lang_invalid_format() {
        let text = "# language: en_US\nSome content";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_extract_lang_empty_text() {
        let text = "";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_extract_lang_multiline() {
        let text = "# First line\n// lang: de_DE\nThird line";
        assert_eq!(extract_lang(text), Some("de_DE".to_string()));
    }

    #[test]
    fn test_extract_lang_with_underscore() {
        let text = "# lang: pt_BR\nSome content";
        assert_eq!(extract_lang(text), Some("pt_BR".to_string()));
    }

    #[test]
    fn test_extract_lang_case_sensitive() {
        let text = "# lang: EN_us\nSome content";
        assert_eq!(extract_lang(text), Some("EN_us".to_string()));
    }

    #[test]
    fn test_load_dict_nonexistent_language() {
        // Test with a language that definitely doesn't exist
        let result = load_dict("nonexistent_lang_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_load_dict_empty_string() {
        let result = load_dict("");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_lang_only_lang_keyword() {
        let text = "# lang\nSome content";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_extract_lang_with_special_characters() {
        let text = "# lang: en-US\nSome content";
        assert_eq!(extract_lang(text), Some("en".to_string())); // Matches 'en' part before hyphen
    }

    #[test]
    fn test_extract_lang_multiple_matches_first_wins() {
        let text = "# lang: en_US\n// lang: de_DE";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_no_colon() {
        let text = "# lang en_US\nSome content";
        assert_eq!(extract_lang(text), None);
    }

    #[test]
    fn test_extract_lang_with_leading_spaces() {
        let text = "    // lang: it_IT\nSome content";
        assert_eq!(extract_lang(text), Some("it_IT".to_string()));
    }

    #[test]
    fn test_extract_lang_comment_only() {
        let text = "# lang: en_US";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_carriage_return() {
        let text = "# lang: en_US\r\nSome content";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_between_content() {
        let text = "Some content\n# lang: fr_FR\nMore content";
        assert_eq!(extract_lang(text), Some("fr_FR".to_string()));
    }

    #[test]
    fn test_extract_lang_with_tabs() {
        let text = "\t// lang: nl_NL\nSome content";
        assert_eq!(extract_lang(text), Some("nl_NL".to_string()));
    }

    #[test]
    fn test_extract_lang_mixed_case_comment_chars() {
        // Test that it only recognizes the exact comment characters
        let text = "# lang: en_US\nSome content";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_load_dict_with_special_chars() {
        // Test that special characters in language code are handled safely
        let result = load_dict("../etc/passwd");
        assert!(result.is_none()); // Should not load arbitrary files
    }

    #[test]
    fn test_extract_lang_language_only() {
        let text = "# lang: en\nSome content";
        assert_eq!(extract_lang(text), Some("en".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_text_in_sentence() {
        let text = "Some text lang: en_US here";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_text_at_start() {
        let text = "lang: de_DE Some content here";
        assert_eq!(extract_lang(text), Some("de_DE".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_text_middle() {
        let text = "This is lang: fr_FR in the middle";
        assert_eq!(extract_lang(text), Some("fr_FR".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_text_end() {
        let text = "Some content here lang: es_ES";
        assert_eq!(extract_lang(text), Some("es_ES".to_string()));
    }

    #[test]
    fn test_extract_lang_comment_before_plain() {
        let text = "# lang: en_US\nSome text lang: de_DE";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_plain_before_comment() {
        let text = "lang: en_US\n# lang: de_DE";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_multiple_html_comments_first_wins() {
        let text = "<!-- lang: en_US -->\nSome content\n<!-- lang: de_DE -->";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_multiple_hash_comments_first_wins() {
        let text = "# lang: en_US\nSome content\n# lang: de_DE";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_mixed_comment_styles_first_wins() {
        let text = "# lang: en_US\nSome content\n<!-- lang: de_DE -->";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }

    #[test]
    fn test_extract_lang_html_before_hash() {
        let text = "<!-- lang: en_US -->\n# lang: de_DE\nSome content";
        assert_eq!(extract_lang(text), Some("en_US".to_string()));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use hunspell_rs::CheckResult;

    #[test]
    fn test_german_encoding() {
        let result = std::panic::catch_unwind(|| {
            Hunspell::new("/usr/share/hunspell/de_DE_frami.aff", "/usr/share/hunspell/de_DE_frami.dic")
        });

        assert!(result.is_ok(), "Failed to create Hunspell instance: {:?}", result.err());

        let dict = result.unwrap();

        // Test common German words with umlauts
        let test_words = vec![
            "möchte",   // would like
            "Götter",   // gods
            "für",      // for
            "über",     // over
            "Äpfel",    // apples (capitalized)
        ];

        for word in test_words {
            let result = dict.check(word);
            println!("Testing '{}': {:?}", word, result);
            assert_eq!(result, CheckResult::FoundInDictionary, "Word '{}' should be valid", word);
        }
    }

    #[test]
    fn test_german_suggestions() {
        let dict = Hunspell::new("/usr/share/hunspell/de_DE_frami.aff", "/usr/share/hunspell/de_DE_frami.dic");

        // Test that German umlaut words are recognized correctly
        assert_eq!(dict.check("möchte"), CheckResult::FoundInDictionary, "Word 'möchte' should be valid");
        assert_eq!(dict.check("Götter"), CheckResult::FoundInDictionary, "Word 'Götter' should be valid");
        assert_eq!(dict.check("für"), CheckResult::FoundInDictionary, "Word 'für' should be valid");
        assert_eq!(dict.check("über"), CheckResult::FoundInDictionary, "Word 'über' should be valid");

        // Test suggestions for misspelled words
        let suggestions = dict.suggest("mochte");
        println!("Suggestions for 'mochte': {:?}", suggestions);
        assert!(!suggestions.is_empty(), "Should have suggestions for 'mochte'");
    }
}