use regex::Regex;
use rusqlite::{Connection, Result as SqliteResult};

#[derive(Debug, Clone)]
pub enum LinkType {
    Wiki,
    Markdown,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Wiki => "wiki",
            LinkType::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedLink {
    pub target: String,
    pub link_type: LinkType,
    pub alias: Option<String>,
}

/// Парсит все ссылки из markdown контента
/// Игнорирует ссылки внутри code blocks
pub fn parse_links(content: &str) -> Vec<ParsedLink> {
    let mut links = Vec::new();

    // Убираем code blocks из контента для парсинга
    let content_without_code = remove_code_blocks(content);

    // Wiki-links: [[target]] или [[target|alias]]
    let wiki_re = Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]").unwrap();
    for cap in wiki_re.captures_iter(&content_without_code) {
        let target = cap.get(1).unwrap().as_str().trim().to_string();
        let alias = cap.get(2).map(|m| m.as_str().trim().to_string());

        if !target.is_empty() {
            links.push(ParsedLink {
                target,
                link_type: LinkType::Wiki,
                alias,
            });
        }
    }

    // Markdown links: [text](path.md)
    let md_re = Regex::new(r"\[([^\]]+)\]\(([^\)]+\.md)\)").unwrap();
    for cap in md_re.captures_iter(&content_without_code) {
        let target = cap.get(2).unwrap().as_str().trim().to_string();
        let alias = Some(cap.get(1).unwrap().as_str().trim().to_string());

        // Игнорируем внешние ссылки (http/https)
        if !target.starts_with("http://") && !target.starts_with("https://") {
            links.push(ParsedLink {
                target,
                link_type: LinkType::Markdown,
                alias,
            });
        }
    }

    links
}

/// Удаляет code blocks из контента (``` ... ```)
fn remove_code_blocks(content: &str) -> String {
    let code_block_re = Regex::new(r"```[\s\S]*?```").unwrap();
    let inline_code_re = Regex::new(r"`[^`]+`").unwrap();

    let without_blocks = code_block_re.replace_all(content, "");
    inline_code_re.replace_all(&without_blocks, "").to_string()
}

/// Обновляет ссылки в БД для указанной заметки
pub fn update_links(db: &Connection, source_path: &str, links: &[ParsedLink]) -> SqliteResult<()> {
    // Удаляем старые ссылки
    db.execute(
        "DELETE FROM links WHERE source = ?1",
        [source_path],
    )?;

    // Вставляем новые ссылки
    for link in links {
        db.execute(
            "INSERT INTO links (source, target, link_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![source_path, &link.target, link.link_type.as_str()],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wiki_links() {
        let content = "This is [[Note 1]] and [[Note 2|custom text]].";
        let links = parse_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "Note 1");
        assert_eq!(links[0].alias, None);
        assert_eq!(links[1].target, "Note 2");
        assert_eq!(links[1].alias, Some("custom text".to_string()));
    }

    #[test]
    fn test_parse_markdown_links() {
        let content = "See [other note](other.md) and [another](path/to/note.md).";
        let links = parse_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "other.md");
        assert_eq!(links[1].target, "path/to/note.md");
    }

    #[test]
    fn test_ignore_code_blocks() {
        let content = r#"
Some text [[Real Link]]

```
Code block with [[Fake Link]]
```

Another `[[Inline Code]]` and [[Another Real]].
"#;
        let links = parse_links(content);

        // Должны найтись только Real Link и Another Real
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "Real Link");
        assert_eq!(links[1].target, "Another Real");
    }
}
