use scraper::{Html, Selector};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WebPageResult {
    pub title: String,
    pub text: String,
    pub url: String,
    pub word_count: usize,
}

/// Parse web page and extract main content
pub async fn parse_web_page(url: &str) -> Result<WebPageResult, String> {
    // Fetch HTML content
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse HTML
    let document = Html::parse_document(&html);

    // Extract title
    let title = extract_title(&document);

    // Extract main content
    let text = extract_content(&document)?;

    // Calculate word count
    let word_count = text.split_whitespace().count();

    if word_count < 50 {
        return Err("Could not extract sufficient content from this page (less than 50 words)".to_string());
    }

    Ok(WebPageResult {
        title,
        text,
        url: url.to_string(),
        word_count,
    })
}

/// Extract page title
fn extract_title(document: &Html) -> String {
    // Try <title> tag first
    if let Ok(selector) = Selector::parse("title") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                return title;
            }
        }
    }

    // Try og:title meta tag
    if let Ok(selector) = Selector::parse("meta[property='og:title']") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let title = content.trim().to_string();
                if !title.is_empty() {
                    return title;
                }
            }
        }
    }

    // Try first <h1>
    if let Ok(selector) = Selector::parse("h1") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                return title;
            }
        }
    }

    "Untitled Page".to_string()
}

/// Extract main content from page
fn extract_content(document: &Html) -> Result<String, String> {
    // Remove unwanted elements
    let unwanted_selectors = vec![
        "script", "style", "nav", "header", "footer", "aside",
        "iframe", "noscript", "form", "button",
        ".advertisement", ".ads", ".social-share",
        "#cookie-notice", "#comments", ".comment"
    ];

    let mut text_parts = Vec::new();

    // Try to find main content container
    let content_selectors = vec![
        "article",
        "main",
        "[role='main']",
        ".article-content",
        ".post-content",
        ".entry-content",
        ".content",
        "body"
    ];

    let mut found_content = false;

    for selector_str in content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                // Extract text from headings, paragraphs, lists
                let content = extract_text_from_element(&element, &unwanted_selectors);
                if !content.is_empty() {
                    text_parts.push(content);
                    found_content = true;
                    break;
                }
            }
            if found_content {
                break;
            }
        }
    }

    if text_parts.is_empty() {
        return Err("Could not extract content from this page".to_string());
    }

    Ok(text_parts.join("\n\n"))
}

/// Extract text from an element, preserving structure
fn extract_text_from_element(element: &scraper::ElementRef, unwanted_selectors: &[&str]) -> String {
    let mut result = Vec::new();

    // Parse heading selectors
    let heading_selectors: Vec<Selector> = (1..=6)
        .map(|i| Selector::parse(&format!("h{}", i)).unwrap())
        .collect();

    let p_selector = Selector::parse("p").ok();
    let li_selector = Selector::parse("li").ok();
    let blockquote_selector = Selector::parse("blockquote").ok();

    // Extract headings
    for (level, selector) in heading_selectors.iter().enumerate() {
        for heading in element.select(selector) {
            if !is_in_unwanted(heading, unwanted_selectors) {
                let text = clean_text(&heading.text().collect::<String>());
                if !text.is_empty() {
                    let prefix = "#".repeat(level + 1);
                    result.push(format!("{} {}", prefix, text));
                }
            }
        }
    }

    // Extract paragraphs
    if let Some(selector) = p_selector {
        for p in element.select(&selector) {
            if !is_in_unwanted(p, unwanted_selectors) {
                let text = clean_text(&p.text().collect::<String>());
                if !text.is_empty() && text.len() > 20 {
                    // Skip very short paragraphs
                    result.push(text);
                }
            }
        }
    }

    // Extract list items
    if let Some(selector) = li_selector {
        for li in element.select(&selector) {
            if !is_in_unwanted(li, unwanted_selectors) {
                let text = clean_text(&li.text().collect::<String>());
                if !text.is_empty() {
                    result.push(format!("- {}", text));
                }
            }
        }
    }

    // Extract blockquotes
    if let Some(selector) = blockquote_selector {
        for quote in element.select(&selector) {
            if !is_in_unwanted(quote, unwanted_selectors) {
                let text = clean_text(&quote.text().collect::<String>());
                if !text.is_empty() {
                    result.push(format!("> {}", text));
                }
            }
        }
    }

    result.join("\n\n")
}

/// Check if element is inside an unwanted container
fn is_in_unwanted(element: scraper::ElementRef, unwanted_selectors: &[&str]) -> bool {
    for selector_str in unwanted_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            // Check if element itself matches
            if element.value().name() == selector_str.trim_start_matches('.').trim_start_matches('#') {
                return true;
            }

            // Check if any parent matches
            let mut current = element;
            loop {
                if let Some(parent) = current.parent() {
                    if let Some(parent_element) = parent.value().as_element() {
                        // Check class names
                        if let Some(classes) = parent_element.attr("class") {
                            for class in classes.split_whitespace() {
                                if selector_str == &format!(".{}", class) {
                                    return true;
                                }
                            }
                        }
                        // Check id
                        if let Some(id) = parent_element.attr("id") {
                            if selector_str == &format!("#{}", id) {
                                return true;
                            }
                        }
                        // Check tag name
                        if parent_element.name() == *selector_str {
                            return true;
                        }
                    }

                    if let Some(parent_elem) = parent.value().as_element() {
                        current = scraper::ElementRef::wrap(parent).unwrap();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }
    false
}

/// Clean and normalize text
fn clean_text(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let input = "  Hello   \n  World  \n\n  Test  ";
        assert_eq!(clean_text(input), "Hello World Test");
    }
}
