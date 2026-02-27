use scraper::{Html, Selector, ElementRef};

use crate::errors::{BosuaError, Result};

/// Parse an HTML string into a document.
pub fn parse(html: &str) -> Html {
    Html::parse_document(html)
}

/// Parse an HTML fragment (not a full document).
pub fn parse_fragment(html: &str) -> Html {
    Html::parse_fragment(html)
}

/// Select all elements matching a CSS selector from a parsed document.
pub fn select<'a>(document: &'a Html, selector: &str) -> Result<Vec<ElementRef<'a>>> {
    let sel = Selector::parse(selector)
        .map_err(|e| BosuaError::Application(format!("Invalid CSS selector '{}': {:?}", selector, e)))?;
    Ok(document.select(&sel).collect())
}

/// Select the first element matching a CSS selector.
pub fn select_one<'a>(document: &'a Html, selector: &str) -> Result<Option<ElementRef<'a>>> {
    let sel = Selector::parse(selector)
        .map_err(|e| BosuaError::Application(format!("Invalid CSS selector '{}': {:?}", selector, e)))?;
    Ok(document.select(&sel).next())
}

/// Extract the inner text content from an element.
pub fn text(element: &ElementRef) -> String {
    element.text().collect::<Vec<_>>().join("")
}

/// Extract an attribute value from an element.
pub fn attr<'a>(element: &'a ElementRef, name: &str) -> Option<&'a str> {
    element.value().attr(name)
}

/// Find all links (href attributes) from anchor tags in a document.
pub fn find_links(document: &Html) -> Result<Vec<String>> {
    let anchors = select(document, "a")?;
    Ok(anchors
        .iter()
        .filter_map(|el| attr(el, "href").map(String::from))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"
        <html>
        <body>
            <h1>Title</h1>
            <p class="intro">Hello, world!</p>
            <p class="content">Some content here.</p>
            <a href="https://example.com">Example</a>
            <a href="/about">About</a>
            <a>No href</a>
            <div id="main" data-value="42">
                <span>Nested text</span>
            </div>
        </body>
        </html>
    "#;

    #[test]
    fn test_parse_and_select() {
        let doc = parse(SAMPLE_HTML);
        let paragraphs = select(&doc, "p").unwrap();
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn test_select_by_class() {
        let doc = parse(SAMPLE_HTML);
        let intro = select(&doc, "p.intro").unwrap();
        assert_eq!(intro.len(), 1);
        assert_eq!(text(&intro[0]), "Hello, world!");
    }

    #[test]
    fn test_select_one() {
        let doc = parse(SAMPLE_HTML);
        let h1 = select_one(&doc, "h1").unwrap();
        assert!(h1.is_some());
        assert_eq!(text(&h1.unwrap()), "Title");
    }

    #[test]
    fn test_select_one_missing() {
        let doc = parse(SAMPLE_HTML);
        let result = select_one(&doc, "table").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_attr_extraction() {
        let doc = parse(SAMPLE_HTML);
        let div = select_one(&doc, "div#main").unwrap().unwrap();
        assert_eq!(attr(&div, "id"), Some("main"));
        assert_eq!(attr(&div, "data-value"), Some("42"));
        assert_eq!(attr(&div, "missing"), None);
    }

    #[test]
    fn test_find_links() {
        let doc = parse(SAMPLE_HTML);
        let links = find_links(&doc).unwrap();
        assert_eq!(links, vec!["https://example.com", "/about"]);
    }

    #[test]
    fn test_text_nested() {
        let doc = parse(SAMPLE_HTML);
        let div = select_one(&doc, "div#main").unwrap().unwrap();
        let content = text(&div);
        assert!(content.contains("Nested text"));
    }

    #[test]
    fn test_invalid_selector() {
        let doc = parse(SAMPLE_HTML);
        let result = select(&doc, "[[[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_fragment() {
        let frag = parse_fragment("<p>Just a paragraph</p>");
        let p = select_one(&frag, "p").unwrap().unwrap();
        assert_eq!(text(&p), "Just a paragraph");
    }

    #[test]
    fn test_empty_document() {
        let doc = parse("");
        let links = find_links(&doc).unwrap();
        assert!(links.is_empty());
    }
}
