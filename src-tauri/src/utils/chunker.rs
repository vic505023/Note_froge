use crate::services::doc_parser::PagedText;

/// Represents a text chunk with its index and optional page number
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub index: usize,
    pub text: String,
    pub page: Option<u32>,  // Page/slide number (1-based), None if not applicable
}

/// Split text into chunks with overlap
///
/// Strategy:
/// 1. Split by double newline (paragraphs)
/// 2. If paragraph > chunk_size, split by sentence boundaries
/// 3. Add overlap between chunks
/// 4. Filter out chunks < 100 chars
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    // Split by paragraphs
    let paragraphs: Vec<&str> = text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();

    let mut paragraph_buffer = Vec::new();

    for paragraph in paragraphs {
        let para_trimmed = paragraph.trim();

        // If single paragraph is too large, split by sentences
        if para_trimmed.len() > chunk_size {
            // First, flush current buffer if any
            if !paragraph_buffer.is_empty() {
                let chunk_text = paragraph_buffer.join("\n\n");
                if chunk_text.len() >= 100 {
                    chunks.push(TextChunk {
                        index: chunk_index,
                        text: chunk_text,
                        page: None,
                    });
                    chunk_index += 1;
                }
                paragraph_buffer.clear();
            }

            // Split large paragraph by sentences
            let sentences = split_sentences(para_trimmed);
            let mut sentence_buffer = Vec::new();
            let mut sentence_length = 0;

            for sentence in sentences {
                if sentence_length + sentence.len() > chunk_size && !sentence_buffer.is_empty() {
                    // Flush sentence buffer
                    let chunk_text = sentence_buffer.join(" ");
                    if chunk_text.len() >= 100 {
                        chunks.push(TextChunk {
                            index: chunk_index,
                            text: chunk_text,
                            page: None,
                        });
                        chunk_index += 1;
                    }

                    // Keep overlap from previous chunk
                    if overlap > 0 && !sentence_buffer.is_empty() {
                        let overlap_text: String = sentence_buffer.iter()
                            .rev()
                            .scan(0, |len, s: &String| {
                                *len += s.len();
                                if *len <= overlap {
                                    Some(s.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join(" ");

                        sentence_buffer.clear();
                        sentence_buffer.push(overlap_text);
                        sentence_length = sentence_buffer[0].len();
                    } else {
                        sentence_buffer.clear();
                        sentence_length = 0;
                    }
                }

                sentence_buffer.push(sentence.to_string());
                sentence_length += sentence.len();
            }

            // Flush remaining sentences
            if !sentence_buffer.is_empty() {
                let chunk_text = sentence_buffer.join(" ");
                if chunk_text.len() >= 100 {
                    chunks.push(TextChunk {
                        index: chunk_index,
                        text: chunk_text,
                        page: None,
                    });
                    chunk_index += 1;
                }
            }
        } else {
            // Normal paragraph - add to buffer
            let potential_length = paragraph_buffer.iter()
                .map(|p: &String| p.len())
                .sum::<usize>() + para_trimmed.len() + paragraph_buffer.len() * 2; // Account for \n\n

            if potential_length > chunk_size && !paragraph_buffer.is_empty() {
                // Flush buffer
                let chunk_text = paragraph_buffer.join("\n\n");
                if chunk_text.len() >= 100 {
                    chunks.push(TextChunk {
                        index: chunk_index,
                        text: chunk_text,
                        page: None,
                    });
                    chunk_index += 1;
                }

                // Keep overlap
                if overlap > 0 && !paragraph_buffer.is_empty() {
                    let overlap_text: String = paragraph_buffer.iter()
                        .rev()
                        .scan(0, |len, p| {
                            *len += p.len();
                            if *len <= overlap {
                                Some(p.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    paragraph_buffer.clear();
                    paragraph_buffer.push(overlap_text);
                } else {
                    paragraph_buffer.clear();
                }
            }

            paragraph_buffer.push(para_trimmed.to_string());
        }
    }

    // Flush remaining buffer
    if !paragraph_buffer.is_empty() {
        let chunk_text = paragraph_buffer.join("\n\n");
        if chunk_text.len() >= 100 {
            chunks.push(TextChunk {
                index: chunk_index,
                text: chunk_text,
                page: None,
            });
        }
    }

    chunks
}

/// Split paged text into chunks with page numbers preserved
///
/// Each chunk remembers which page it came from. If a chunk spans multiple pages,
/// it's assigned to the page where it starts.
pub fn chunk_pages(pages: &[PagedText], chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
    eprintln!("Chunking {} pages, chunk_size={}, overlap={}", pages.len(), chunk_size, overlap);

    let mut chunks = Vec::new();
    let mut chunk_index = 0;
    let mut current_page_num = 1;
    let mut accumulated_text = String::new();
    let mut accumulated_start_page = 1;

    for page in pages {
        current_page_num = page.page;

        // If accumulated text is empty, this is the start of a new chunk
        if accumulated_text.is_empty() {
            accumulated_start_page = current_page_num;
        }

        // Try adding this page's text
        let combined = if accumulated_text.is_empty() {
            page.text.clone()
        } else {
            format!("{}\n\n{}", accumulated_text, page.text)
        };

        if combined.len() > chunk_size && !accumulated_text.is_empty() {
            // Current accumulated text is enough for a chunk
            let chunk_text = accumulated_text.clone();
            if chunk_text.trim().len() >= 100 {
                chunks.push(TextChunk {
                    index: chunk_index,
                    text: chunk_text.clone(),
                    page: Some(accumulated_start_page),
                });
                chunk_index += 1;
            }

            // Handle overlap
            if overlap > 0 && chunk_text.len() > overlap {
                let overlap_text = &chunk_text[chunk_text.len() - overlap..];
                accumulated_text = overlap_text.to_string();
                accumulated_start_page = current_page_num;
            } else {
                accumulated_text.clear();
                accumulated_start_page = current_page_num;
            }

            // Add current page text
            if !accumulated_text.is_empty() {
                accumulated_text.push_str("\n\n");
            }
            accumulated_text.push_str(&page.text);
        } else {
            accumulated_text = combined;
        }
    }

    // Flush remaining text
    if !accumulated_text.is_empty() && accumulated_text.trim().len() >= 100 {
        chunks.push(TextChunk {
            index: chunk_index,
            text: accumulated_text,
            page: Some(accumulated_start_page),
        });
    }

    eprintln!("Created {} chunks:", chunks.len());
    for chunk in &chunks {
        eprintln!("  Chunk {}: page={:?}, len={}", chunk.index, chunk.page, chunk.text.len());
    }

    chunks
}

/// Split text by sentence boundaries (. ! ? followed by space or newline)
fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;

    for (i, c) in text.char_indices() {
        if c == '.' || c == '!' || c == '?' {
            // Check if followed by space or newline
            if let Some(next_char) = text[i+1..].chars().next() {
                if next_char.is_whitespace() {
                    let end = i + c.len_utf8();
                    let sentence = text[start..end].trim();
                    if !sentence.is_empty() {
                        sentences.push(sentence);
                    }
                    start = end;
                }
            } else {
                // End of text
                let sentence = text[start..].trim();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                break;
            }
        }
    }

    // Add remaining text if any
    if start < text.len() {
        let sentence = text[start..].trim();
        if !sentence.is_empty() {
            sentences.push(sentence);
        }
    }

    // If no sentences found, return entire text
    if sentences.is_empty() {
        vec![text]
    } else {
        sentences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_small_text() {
        let text = "Short text.";
        let chunks = chunk_text(text, 1500, 200);
        assert_eq!(chunks.len(), 0); // Too small, filtered out
    }

    #[test]
    fn test_chunk_paragraphs() {
        let text = "First paragraph with some content.\n\nSecond paragraph with more content.\n\nThird paragraph.";
        let chunks = chunk_text(text, 50, 10);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_large_paragraph() {
        let text = "This is a very long sentence. ".repeat(100);
        let chunks = chunk_text(&text, 500, 100);
        assert!(!chunks.is_empty());
    }
}
