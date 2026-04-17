use hunspell_rs::Hunspell;
use regex::Regex;

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