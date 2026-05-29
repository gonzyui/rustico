use scraper::Html;
use regex::Regex;
use once_cell::sync::Lazy;

static HTML_ENTITY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"&(?:#(\d+)|#x([0-9a-fA-F]+)|([a-zA-Z]+));").unwrap()
});

/// Strips HTML tags and decodes HTML entities from a string
/// Handles tags typically emitted by ANN and AniList
pub fn clean_html(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let html = Html::parse_fragment(s);
    let mut text = String::new();
    extract_text(&html.root_element(), &mut text);
    
    text = text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    
    decode_html_entities(&text)
}

/// Recursively extract text from HTML nodes
fn extract_text(node: &scraper::element_ref::ElementRef, text: &mut String) {
    use scraper::node::Node;
    
    for child in node.children() {
        match child.value() {
            Node::Text(t) => {
                let content = t.trim();
                if !content.is_empty() {
                    text.push_str(content);
                    text.push(' ');
                }
            }
            Node::Element(e) => {
                match e.name() {
                    "br" | "p" => text.push('\n'),
                    "li" => text.push_str("\n• "),
                    _ => {}
                }
                
                if let Some(child_ref) = scraper::element_ref::ElementRef::wrap(child) {
                    extract_text(&child_ref, text);
                }
            }
            _ => {}
        }
    }
}

/// Decode HTML entities (&#123;, &#x7B;, &amp;, etc.)
fn decode_html_entities(s: &str) -> String {
    HTML_ENTITY_REGEX
        .replace_all(s, |caps: &regex::Captures| {
            if let Some(dec) = caps.get(1) {
                if let Ok(code) = dec.as_str().parse::<u32>() {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            } else if let Some(hex) = caps.get(2) {
                if let Ok(code) = u32::from_str_radix(hex.as_str(), 16) {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            } else if let Some(named) = caps.get(3) {
                match named.as_str() {
                    "amp" => return "&".to_string(),
                    "lt" => return "<".to_string(),
                    "gt" => return ">".to_string(),
                    "quot" => return "\"".to_string(),
                    "apos" | "#39" => return "'".to_string(),
                    "nbsp" => return " ".to_string(),
                    _ => {}
                }
            }
            caps[0].to_string()
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_html_basic() {
        let html = "<p>Hello <b>world</b></p>";
        let result = clean_html(html);
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
    }

    #[test]
    fn test_clean_html_entities() {
        let html = "Hello &amp; goodbye &quot;world&quot;";
        let result = clean_html(html);
        assert!(result.contains("&"));
        assert!(result.contains("\""));
    }

    #[test]
    fn test_clean_html_line_breaks() {
        let html = "<p>Line 1</p><p>Line 2</p>";
        let result = clean_html(html);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
    }

    #[test]
    fn test_clean_html_empty() {
        assert_eq!(clean_html(""), "");
    }
}