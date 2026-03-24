//! Shared utility functions.

/// Escape a string for safe inclusion in HTML content.
///
/// Replaces the five characters that have special meaning in HTML:
/// `&`, `<`, `>`, `"`, and `'` with their corresponding HTML entities.
#[must_use]
pub fn html_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape_basic() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(
            html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_html_escape_empty() {
        assert_eq!(html_escape(""), "");
    }

    // ── HEAD tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_html_escape_each_special_char_individually() {
        assert_eq!(html_escape("&"), "&amp;");
        assert_eq!(html_escape("<"), "&lt;");
        assert_eq!(html_escape(">"), "&gt;");
        assert_eq!(html_escape("\""), "&quot;");
        assert_eq!(html_escape("'"), "&#x27;");
    }

    #[test]
    fn test_html_escape_unicode_passthrough() {
        assert_eq!(html_escape("héllo"), "héllo");
        assert_eq!(html_escape("日本語"), "日本語");
        assert_eq!(html_escape("Ñoño"), "Ñoño");
        assert_eq!(html_escape("emoji: 🎉"), "emoji: 🎉");
    }

    #[test]
    fn test_html_escape_repeated_special_chars() {
        assert_eq!(html_escape("&&"), "&amp;&amp;");
        assert_eq!(html_escape("<<>>"), "&lt;&lt;&gt;&gt;");
        assert_eq!(html_escape("\"\""), "&quot;&quot;");
    }

    #[test]
    fn test_html_escape_plain_text_unchanged() {
        let plain = "The quick brown fox jumps over the lazy dog 0123456789";
        assert_eq!(html_escape(plain), plain);
    }

    // ── Copilot tests ───────────────────────────────────────────────────────

    #[test]
    fn test_html_escape_all_five_special_chars() {
        // Each special character must map to its entity
        assert_eq!(html_escape("&"), "&amp;");
        assert_eq!(html_escape("<"), "&lt;");
        assert_eq!(html_escape(">"), "&gt;");
        assert_eq!(html_escape("\""), "&quot;");
        assert_eq!(html_escape("'"), "&#x27;");
    }

    #[test]
    fn test_html_escape_no_special_chars() {
        // Plain alphanumerics and spaces are unchanged
        assert_eq!(html_escape("Hello World 123"), "Hello World 123");
    }

    #[test]
    fn test_html_escape_multiple_consecutive_specials() {
        assert_eq!(html_escape("&&"), "&amp;&amp;");
        assert_eq!(html_escape("<<"), "&lt;&lt;");
    }

    #[test]
    fn test_html_escape_mixed_content() {
        // Realistic HTML injection attempt
        let input = "<img src=x onerror=\"alert('xss')\">";
        let out = html_escape(input);
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(!out.contains('"'));
        assert!(!out.contains('\''));
        assert!(out.contains("&lt;img"));
        assert!(out.contains("&quot;"));
        assert!(out.contains("&#x27;"));
    }

    #[test]
    fn test_html_escape_unicode_passthrough_copilot() {
        // Non-ASCII characters must be preserved as-is
        let input = "Ünïcödé ñ 日本語";
        assert_eq!(html_escape(input), input);
    }

    #[test]
    fn test_html_escape_preserves_newlines_and_tabs() {
        assert_eq!(html_escape("line1\nline2"), "line1\nline2");
        assert_eq!(html_escape("col1\tcol2"), "col1\tcol2");
    }
}
