use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use std::process::Command;
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;
use reqwest::Client;
use lopdf::Document as PdfDocument;
use crate::utils::vision::describe_image;

/// Represents text from a single page/slide
#[derive(Debug, Clone)]
pub struct PagedText {
    pub page: u32,      // 1-based page/slide number
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub title: String,
    pub pages: Vec<PagedText>,
    pub page_count: Option<u32>,
}

impl ParsedDocument {
    /// Get full text by joining all pages (for backward compatibility)
    pub fn full_text(&self) -> String {
        self.pages.iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Render a PDF page as PNG image using pdftoppm
///
/// Requires poppler-utils to be installed (sudo pacman -S poppler)
///
/// # Arguments
/// * `pdf_path` - Path to PDF file
/// * `page_num` - Page number (1-indexed)
/// * `output_dir` - Directory to save PNG (will create page-{N}.png)
///
/// # Returns
/// Path to generated PNG file
fn pdf_page_to_image(pdf_path: &Path, page_num: u32, output_dir: &Path) -> Result<PathBuf, String> {
    // Check if pdftoppm is available
    let check = Command::new("which")
        .arg("pdftoppm")
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        return Err("pdftoppm not found. Install poppler-utils: sudo pacman -S poppler".to_string());
    }

    // Create output directory if needed
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Run pdftoppm to render page
    // pdftoppm -png -f {page} -l {page} -r 200 input.pdf output_prefix
    let output_prefix = output_dir.join("page");
    let output = Command::new("pdftoppm")
        .arg("-png")
        .arg("-f")
        .arg(page_num.to_string())
        .arg("-l")
        .arg(page_num.to_string())
        .arg("-r")
        .arg("200") // 200 DPI for good quality
        .arg(pdf_path)
        .arg(&output_prefix)
        .output()
        .map_err(|e| format!("Failed to run pdftoppm: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftoppm failed: {}", stderr));
    }

    // pdftoppm creates files like "page-1.png", "page-2.png", etc.
    let png_path = output_dir.join(format!("page-{}.png", page_num));

    if !png_path.exists() {
        return Err(format!("Expected output file not found: {}", png_path.display()));
    }

    Ok(png_path)
}

/// Parse PDF file using pdf-extract
pub fn parse_pdf(path: &Path) -> Result<ParsedDocument, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Failed to read PDF: {}", e))?;

    match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => {
            if text.trim().is_empty() {
                return Err("Scanned PDF - no text layer available".to_string());
            }

            // Split by form feed to get pages
            let pages_text: Vec<&str> = text.split('\x0C').collect();
            let page_count = pages_text.len() as u32;

            eprintln!("PDF parsing: {} pages detected by form feed", page_count);
            for (i, page_text) in pages_text.iter().enumerate() {
                eprintln!("  Page {}: {} chars", i + 1, page_text.len());
            }

            // Convert to PagedText
            let pages: Vec<PagedText> = pages_text.into_iter()
                .enumerate()
                .map(|(i, page_text)| PagedText {
                    page: (i + 1) as u32,  // 1-based
                    text: page_text.to_string(),
                })
                .collect();

            // Extract title from first page or filename
            let title = pages.first()
                .and_then(|p| {
                    p.text.lines()
                        .next()
                        .filter(|line| !line.trim().is_empty() && line.len() < 200)
                        .map(|line| line.trim().to_string())
                })
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Untitled")
                        .to_string()
                });

            Ok(ParsedDocument {
                title,
                pages,
                page_count: Some(page_count),
            })
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("password") || err_str.contains("encrypted") {
                Err("Password-protected PDF - cannot extract text".to_string())
            } else {
                Err(format!("PDF parsing error: {}", err_str))
            }
        }
    }
}

/// Parse PDF file with Vision API fallback for scanned pages
///
/// # Arguments
/// * `path` - Path to PDF file
/// * `client` - HTTP client for vision API
/// * `vision_enabled` - Whether vision OCR is enabled
/// * `base_url` - Vision API base URL
/// * `api_key` - Vision API key
/// * `model` - Vision model to use (e.g., "gpt-4o-mini")
///
/// # Returns
/// Parsed document with text extracted from all pages (OCR where needed)
pub async fn parse_pdf_with_vision(
    path: &Path,
    client: &Client,
    vision_enabled: bool,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<ParsedDocument, String> {
    eprintln!(">>> parse_pdf_with_vision called");
    eprintln!("    Path: {}", path.display());
    eprintln!("    Vision enabled: {}", vision_enabled);
    eprintln!("    Model: {}", model);
    eprintln!("    Base URL: {}", base_url);

    let bytes = fs::read(path)
        .map_err(|e| format!("Failed to read PDF: {}", e))?;

    eprintln!("    PDF size: {} bytes", bytes.len());

    // Get page count from PDF structure
    let total_pages = match PdfDocument::load_mem(&bytes) {
        Ok(doc) => doc.get_pages().len() as u32,
        Err(_) => {
            eprintln!("Failed to read PDF structure, assuming 1 page");
            1
        }
    };
    eprintln!("    PDF has {} pages (from structure)", total_pages);

    // First try regular extraction
    let extracted_text = match pdf_extract::extract_text_from_mem(&bytes) {
        Ok(text) => text,
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("password") || err_str.contains("encrypted") {
                return Err("Password-protected PDF - cannot extract text".to_string());
            } else {
                // If extraction completely fails, treat as empty (will try vision)
                eprintln!("PDF text extraction failed: {}. Will try vision OCR.", err_str);
                String::new()
            }
        }
    };

    // Split by form feed to get pages
    let pages_text: Vec<&str> = if !extracted_text.is_empty() {
        extracted_text.split('\x0C').collect()
    } else {
        vec![]
    };

    let page_count = if pages_text.is_empty() {
        // PDF has no text at all - it's a scan
        // Use page count from PDF structure
        eprintln!("PDF has no extractable text - treating as scanned document");
        total_pages
    } else {
        pages_text.len() as u32
    };

    // Process each page - use vision if text is too short
    const MIN_TEXT_THRESHOLD: usize = 100;
    let mut all_pages: Vec<PagedText> = Vec::new();
    let temp_dir = std::env::temp_dir().join(format!("noteforge_pdf_{}", uuid::Uuid::new_v4()));

    // If PDF has no pages at all (scanned), process all pages with vision
    if pages_text.is_empty() && vision_enabled && !api_key.is_empty() {
        eprintln!("No text pages found, attempting vision OCR on all {} pages", page_count);
        for page_num in 1..=page_count {
            match pdf_page_to_image(path, page_num, &temp_dir) {
                Ok(image_path) => {
                    eprintln!("PDF page {} rendered to image: {}", page_num, image_path.display());
                    let context = format!("Page {} of {} in PDF document (scanned)", page_num, page_count);
                    eprintln!("Calling Vision API for page {} with model: {} at {}", page_num, model, base_url);
                    match describe_image(client, base_url, api_key, model, &image_path, &context).await {
                        Ok(vision_text) => {
                            eprintln!("✓ Vision OCR succeeded for page {}, got {} chars", page_num, vision_text.len());
                            all_pages.push(PagedText {
                                page: page_num,
                                text: vision_text,
                            });
                        }
                        Err(e) => {
                            eprintln!("✗ Vision OCR FAILED for page {}: {}", page_num, e);
                            eprintln!("  Model: {}, Base URL: {}", model, base_url);
                            eprintln!("  API key present: {}", !api_key.is_empty());
                            all_pages.push(PagedText {
                                page: page_num,
                                text: String::new(),
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to render page {}: {}", page_num, e);
                    all_pages.push(PagedText {
                        page: page_num,
                        text: String::new(),
                    });
                }
            }
        }
    }

    for (i, page_text) in pages_text.iter().enumerate() {
        let page_num = (i + 1) as u32;
        let text_content = page_text.trim();

        if text_content.len() >= MIN_TEXT_THRESHOLD {
            // Enough text, use as-is
            all_pages.push(PagedText {
                page: page_num,
                text: text_content.to_string(),
            });
        } else if vision_enabled && !api_key.is_empty() {
            // Too little text - try vision OCR
            eprintln!("Page {} has only {} chars, attempting vision OCR", page_num, text_content.len());

            match pdf_page_to_image(path, page_num, &temp_dir) {
                Ok(image_path) => {
                    eprintln!("PDF page {} rendered to image: {}", page_num, image_path.display());
                    let context = format!("Page {} of PDF document", page_num);
                    eprintln!("Calling Vision API with model: {} at {}", model, base_url);
                    match describe_image(client, base_url, api_key, model, &image_path, &context).await {
                        Ok(vision_text) => {
                            eprintln!("✓ Vision OCR succeeded for page {}, got {} chars", page_num, vision_text.len());
                            all_pages.push(PagedText {
                                page: page_num,
                                text: vision_text,
                            });
                        }
                        Err(e) => {
                            eprintln!("✗ Vision OCR FAILED for page {}: {}", page_num, e);
                            eprintln!("  Model: {}, Base URL: {}", model, base_url);
                            eprintln!("  API key present: {}", !api_key.is_empty());
                            // Fall back to whatever text we have
                            all_pages.push(PagedText {
                                page: page_num,
                                text: text_content.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to render page {}: {}", page_num, e);
                    // Fall back to text extraction
                    all_pages.push(PagedText {
                        page: page_num,
                        text: text_content.to_string(),
                    });
                }
            }
        } else {
            // Vision disabled or no API key - use what we have
            all_pages.push(PagedText {
                page: page_num,
                text: text_content.to_string(),
            });
        }
    }

    // Clean up temp directory
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // Check if all pages are empty
    let has_text = all_pages.iter().any(|p| !p.text.trim().is_empty());

    if !has_text {
        eprintln!("✗ PDF parsing failed: all pages returned empty text");
        eprintln!("  Vision enabled: {}", vision_enabled);
        eprintln!("  Total pages processed: {}", all_pages.len());
        eprintln!("  API key present: {}", !api_key.is_empty());

        if vision_enabled && !api_key.is_empty() {
            return Err("PDF has no extractable text and Vision OCR failed to extract content. Check terminal logs for Vision API errors.".to_string());
        } else {
            return Err("PDF has no extractable text. Enable Vision OCR in Settings to process scanned documents.".to_string());
        }
    }

    // Extract title from first page or filename
    let title = all_pages.first()
        .and_then(|page| {
            page.text.lines()
                .find(|line| !line.trim().is_empty())
                .filter(|line| line.len() < 200)
                .map(|line| line.trim().to_string())
        })
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    Ok(ParsedDocument {
        title,
        pages: all_pages,
        page_count: Some(page_count),
    })
}

/// Parse DOCX file (Office Open XML)
pub fn parse_docx(path: &Path) -> Result<ParsedDocument, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open DOCX: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid DOCX file: {}", e))?;

    // Extract word/document.xml
    let mut document_xml = archive.by_name("word/document.xml")
        .map_err(|e| format!("DOCX missing document.xml: {}", e))?;

    let mut xml_content = String::new();
    document_xml.read_to_string(&mut xml_content)
        .map_err(|e| format!("Failed to read document.xml: {}", e))?;

    // Parse XML and extract text from <w:t> elements
    let mut reader = Reader::from_str(&xml_content);
    reader.config_mut().trim_text(true);

    let mut text_parts = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut in_text_element = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"w:t" => {
                in_text_element = true;
            }
            Ok(Event::Text(e)) if in_text_element => {
                if let Ok(txt) = e.unescape() {
                    current_paragraph.push(txt.to_string());
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:t" => {
                in_text_element = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:p" => {
                // End of paragraph
                if !current_paragraph.is_empty() {
                    text_parts.push(current_paragraph.join(""));
                    current_paragraph.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parsing error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // Join paragraphs with newlines
    let text = text_parts.join("\n");

    if text.trim().is_empty() {
        return Err("DOCX file is empty or has no text".to_string());
    }

    // Title is first non-empty line or filename
    let title = text.lines()
        .find(|line| !line.trim().is_empty())
        .filter(|line| line.len() < 200)
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    // DOCX doesn't have explicit pages - treat as single page
    // TODO: Could detect page breaks (<w:br w:type="page"/>) in future
    let pages = vec![PagedText {
        page: 1,
        text,
    }];

    Ok(ParsedDocument {
        title,
        pages,
        page_count: None,
    })
}

/// Parse DOCX file with Vision API fallback for scanned documents
///
/// # Arguments
/// * `path` - Path to DOCX file
/// * `client` - HTTP client for vision API
/// * `vision_enabled` - Whether vision OCR is enabled
/// * `base_url` - Vision API base URL
/// * `api_key` - Vision API key
/// * `model` - Vision model to use
///
/// # Returns
/// Parsed document with text (OCR for scanned docs with embedded images)
pub async fn parse_docx_with_vision(
    path: &Path,
    client: &Client,
    vision_enabled: bool,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<ParsedDocument, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open DOCX: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid DOCX file: {}", e))?;

    // Extract word/document.xml
    let mut document_xml = archive.by_name("word/document.xml")
        .map_err(|e| format!("DOCX missing document.xml: {}", e))?;

    let mut xml_content = String::new();
    document_xml.read_to_string(&mut xml_content)
        .map_err(|e| format!("Failed to read document.xml: {}", e))?;

    // Parse XML and extract text from <w:t> elements
    let mut reader = Reader::from_str(&xml_content);
    reader.config_mut().trim_text(true);

    let mut text_parts = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut in_text_element = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"w:t" => {
                in_text_element = true;
            }
            Ok(Event::Text(e)) if in_text_element => {
                if let Ok(txt) = e.unescape() {
                    current_paragraph.push(txt.to_string());
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:t" => {
                in_text_element = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"w:p" => {
                // End of paragraph
                if !current_paragraph.is_empty() {
                    text_parts.push(current_paragraph.join(""));
                    current_paragraph.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parsing error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // Join paragraphs with newlines
    let mut text = text_parts.join("\n");

    // Check if document is mostly empty (scanned doc)
    const MIN_TEXT_THRESHOLD: usize = 200;

    if text.trim().len() < MIN_TEXT_THRESHOLD && vision_enabled && !api_key.is_empty() {
        eprintln!("DOCX has only {} chars, attempting vision OCR on embedded images", text.len());

        // Extract all images from word/media/
        let mut images = Vec::new();
        let mut archive_reopen = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();

        for i in 0..archive_reopen.len() {
            let mut file = archive_reopen.by_index(i).ok();
            if let Some(ref mut f) = file {
                let name = f.name().to_string();
                if name.starts_with("word/media/") {
                    let ext = name.split('.').last().unwrap_or("").to_lowercase();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp") {
                        let mut img_bytes = Vec::new();
                        if f.read_to_end(&mut img_bytes).is_ok() && img_bytes.len() > 10 * 1024 {
                            let filename = name.split('/').last().unwrap_or("image").to_string();
                            images.push((filename, img_bytes));
                        }
                    }
                }
            }
        }

        // Sort by size, largest first
        images.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        if !images.is_empty() {
            let temp_dir = std::env::temp_dir().join(format!("noteforge_docx_{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&temp_dir).ok();

            let mut ocr_texts = Vec::new();

            // Process up to 5 largest images
            for (idx, (filename, img_bytes)) in images.iter().take(5).enumerate() {
                let img_path = temp_dir.join(filename);
                if fs::write(&img_path, img_bytes).is_ok() {
                    let context = format!("Image {} from DOCX document", idx + 1);
                    match describe_image(client, base_url, api_key, model, &img_path, &context).await {
                        Ok(vision_text) => {
                            ocr_texts.push(format!("[Image {} - Vision OCR]\n{}", idx + 1, vision_text));
                        }
                        Err(e) => {
                            eprintln!("Vision OCR failed for image {}: {}", idx + 1, e);
                        }
                    }
                }
            }

            // Clean up temp directory
            let _ = fs::remove_dir_all(&temp_dir);

            // Combine original text with OCR
            if !ocr_texts.is_empty() {
                if !text.is_empty() {
                    text.push_str("\n\n");
                }
                text.push_str(&ocr_texts.join("\n\n"));
            }
        }
    }

    if text.trim().is_empty() {
        return Err("DOCX file is empty or has no text".to_string());
    }

    // Title is first non-empty line or filename
    let title = text.lines()
        .find(|line| !line.trim().is_empty() && !line.contains("[Image") && !line.contains("Vision OCR"))
        .filter(|line| line.len() < 200)
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    // DOCX treated as single page
    let pages = vec![PagedText {
        page: 1,
        text,
    }];

    Ok(ParsedDocument {
        title,
        pages,
        page_count: None,
    })
}

/// Extract embedded images from a PPTX slide
///
/// Returns the largest image (>10KB) from the slide's media folder
///
/// # Arguments
/// * `zip_path` - Path to PPTX file
/// * `slide_num` - Slide number (1-indexed)
///
/// # Returns
/// Vector of (filename, image_bytes) for images in this slide
fn extract_slide_images(zip_path: &Path, slide_num: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open PPTX: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid PPTX file: {}", e))?;

    // Read slide relationships to find referenced images
    let rels_name = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);

    let mut image_targets = Vec::new();

    if let Ok(mut rels_file) = archive.by_name(&rels_name) {
        let mut rels_content = String::new();
        rels_file.read_to_string(&mut rels_content).ok();

        // Parse relationships XML to find image references
        let mut reader = Reader::from_str(&rels_content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"Relationship" => {
                    let mut is_image = false;
                    let mut target = String::new();

                    for attr in e.attributes().filter_map(Result::ok) {
                        if attr.key.as_ref() == b"Type" {
                            let value = String::from_utf8_lossy(&attr.value);
                            if value.contains("image") {
                                is_image = true;
                            }
                        }
                        if attr.key.as_ref() == b"Target" {
                            target = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }

                    if is_image && !target.is_empty() {
                        // Target is like "../media/image1.png"
                        let image_path = target.replace("../", "ppt/");
                        image_targets.push(image_path);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    // Extract image files
    let mut images = Vec::new();
    for image_path in image_targets {
        if let Ok(mut img_file) = archive.by_name(&image_path) {
            let mut img_bytes = Vec::new();
            if img_file.read_to_end(&mut img_bytes).is_ok() {
                // Only include images larger than 10KB (filter out icons/bullets)
                if img_bytes.len() > 10 * 1024 {
                    let filename = image_path.split('/').last().unwrap_or("image").to_string();
                    images.push((filename, img_bytes));
                }
            }
        }
    }

    // Sort by size, largest first
    images.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    Ok(images)
}

/// Parse PPTX file (Office Open XML)
pub fn parse_pptx(path: &Path) -> Result<ParsedDocument, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open PPTX: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid PPTX file: {}", e))?;

    let mut slides = Vec::new();

    // Find all slide files (ppt/slides/slide*.xml)
    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            // Extract slide number for sorting
            let slide_num = name
                .trim_start_matches("ppt/slides/slide")
                .trim_end_matches(".xml")
                .parse::<u32>()
                .unwrap_or(0);

            let mut content = String::new();
            let mut reader = file;
            reader.read_to_string(&mut content)
                .map_err(|e| format!("Failed to read slide XML: {}", e))?;

            slides.push((slide_num, content));
        }
    }

    if slides.is_empty() {
        return Err("PPTX file has no slides".to_string());
    }

    // Sort slides by number
    slides.sort_by_key(|(num, _)| *num);
    let page_count = slides.len() as u32;

    // Parse each slide and extract text from <a:t> elements
    let mut pages = Vec::new();

    for (slide_num, xml_content) in slides.iter() {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut slide_text = Vec::new();
        let mut in_text_element = false;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"a:t" => {
                    in_text_element = true;
                }
                Ok(Event::Text(e)) if in_text_element => {
                    if let Ok(txt) = e.unescape() {
                        slide_text.push(txt.to_string());
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                    in_text_element = false;
                }
                Ok(Event::Eof) => break,
                Err(_) => break, // Continue on error
                _ => {}
            }
            buf.clear();
        }

        if !slide_text.is_empty() {
            pages.push(PagedText {
                page: *slide_num,
                text: slide_text.join(" "),
            });
        }
    }

    if pages.is_empty() {
        return Err("PPTX file has no text content".to_string());
    }

    // Title is text from first slide or filename
    let title = pages.first()
        .and_then(|p| {
            p.text.lines()
                .find(|line| !line.trim().is_empty())
                .filter(|line| line.len() < 200)
                .map(|line| line.trim().to_string())
        })
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    Ok(ParsedDocument {
        title,
        pages,
        page_count: Some(page_count),
    })
}

/// Parse PPTX file with Vision API fallback for image-heavy slides
///
/// # Arguments
/// * `path` - Path to PPTX file
/// * `client` - HTTP client for vision API
/// * `vision_enabled` - Whether vision OCR is enabled
/// * `base_url` - Vision API base URL
/// * `api_key` - Vision API key
/// * `model` - Vision model to use
///
/// # Returns
/// Parsed document with text from all slides (OCR for image-only slides)
pub async fn parse_pptx_with_vision(
    path: &Path,
    client: &Client,
    vision_enabled: bool,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<ParsedDocument, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open PPTX: {}", e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid PPTX file: {}", e))?;

    let mut slides = Vec::new();

    // Find all slide files (ppt/slides/slide*.xml)
    for i in 0..archive.len() {
        let file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let name = file.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            // Extract slide number for sorting
            let slide_num = name
                .trim_start_matches("ppt/slides/slide")
                .trim_end_matches(".xml")
                .parse::<u32>()
                .unwrap_or(0);

            let mut content = String::new();
            let mut reader = file;
            reader.read_to_string(&mut content)
                .map_err(|e| format!("Failed to read slide XML: {}", e))?;

            slides.push((slide_num, content));
        }
    }

    if slides.is_empty() {
        return Err("PPTX file has no slides".to_string());
    }

    // Sort slides by number
    slides.sort_by_key(|(num, _)| *num);
    let page_count = slides.len() as u32;

    // Parse each slide
    let mut pages = Vec::new();
    const MIN_TEXT_THRESHOLD: usize = 100;
    let temp_dir = std::env::temp_dir().join(format!("noteforge_pptx_{}", uuid::Uuid::new_v4()));

    for (slide_num, xml_content) in slides.iter() {
        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut slide_text = Vec::new();
        let mut in_text_element = false;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"a:t" => {
                    in_text_element = true;
                }
                Ok(Event::Text(e)) if in_text_element => {
                    if let Ok(txt) = e.unescape() {
                        slide_text.push(txt.to_string());
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"a:t" => {
                    in_text_element = false;
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        let text_content = slide_text.join(" ");

        if text_content.len() >= MIN_TEXT_THRESHOLD {
            // Enough text, use as-is
            pages.push(PagedText {
                page: *slide_num,
                text: text_content,
            });
        } else if vision_enabled && !api_key.is_empty() {
            // Too little text - try vision OCR on slide images
            eprintln!("Slide {} has only {} chars, attempting vision OCR", slide_num, text_content.len());

            match extract_slide_images(path, *slide_num) {
                Ok(images) if !images.is_empty() => {
                    // Save first (largest) image to temp and use vision
                    let (filename, img_bytes) = &images[0];
                    let img_path = temp_dir.join(filename);

                    fs::create_dir_all(&temp_dir).ok();
                    if fs::write(&img_path, img_bytes).is_ok() {
                        let context = format!("Slide {} from presentation", slide_num);
                        match describe_image(client, base_url, api_key, model, &img_path, &context).await {
                            Ok(vision_text) => {
                                pages.push(PagedText {
                                    page: *slide_num,
                                    text: vision_text,
                                });
                            }
                            Err(e) => {
                                eprintln!("Vision OCR failed for slide {}: {}", slide_num, e);
                                if !text_content.is_empty() {
                                    pages.push(PagedText {
                                        page: *slide_num,
                                        text: text_content.clone(),
                                    });
                                }
                            }
                        }
                    } else {
                        if !text_content.is_empty() {
                            pages.push(PagedText {
                                page: *slide_num,
                                text: text_content.clone(),
                            });
                        }
                    }
                }
                _ => {
                    // No images or extraction failed
                    if !text_content.is_empty() {
                        pages.push(PagedText {
                            page: *slide_num,
                            text: text_content.clone(),
                        });
                    } else {
                        eprintln!("Slide {} has no text and no images", slide_num);
                        pages.push(PagedText {
                            page: *slide_num,
                            text: "[No content]".to_string(),
                        });
                    }
                }
            }
        } else {
            // Vision disabled - use what we have
            if !text_content.is_empty() {
                pages.push(PagedText {
                    page: *slide_num,
                    text: text_content,
                });
            } else {
                pages.push(PagedText {
                    page: *slide_num,
                    text: "[No text]".to_string(),
                });
            }
        }
    }

    // Clean up temp directory
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    if pages.is_empty() {
        return Err("PPTX file has no text content".to_string());
    }

    // Title is text from first slide or filename
    let title = pages.first()
        .and_then(|p| {
            p.text.lines()
                .find(|line| !line.trim().is_empty())
                .filter(|line| line.len() < 200)
                .map(|line| line.trim().to_string())
        })
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    Ok(ParsedDocument {
        title,
        pages,
        page_count: Some(page_count),
    })
}

/// Parse plain text file
pub fn parse_txt(path: &Path) -> Result<ParsedDocument, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read TXT: {}", e))?;

    if text.trim().is_empty() {
        return Err("TXT file is empty".to_string());
    }

    // Title is first line or filename
    let title = text.lines()
        .next()
        .filter(|line| !line.trim().is_empty() && line.len() < 200)
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        });

    // TXT files are treated as single page
    let pages = vec![PagedText {
        page: 1,
        text,
    }];

    Ok(ParsedDocument {
        title,
        pages,
        page_count: None,
    })
}

/// Parse document based on extension
pub fn parse_document(path: &Path) -> Result<ParsedDocument, String> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "File has no extension".to_string())?
        .to_lowercase();

    match extension.as_str() {
        "pdf" => parse_pdf(path),
        "docx" => parse_docx(path),
        "pptx" => parse_pptx(path),
        "txt" => parse_txt(path),
        _ => Err(format!("Unsupported file type: {}", extension)),
    }
}

/// Parse document with Vision API fallback (async version)
///
/// # Arguments
/// * `path` - Path to document file
/// * `client` - HTTP client for vision API
/// * `vision_enabled` - Whether vision OCR is enabled
/// * `base_url` - Vision API base URL
/// * `api_key` - Vision API key
/// * `model` - Vision model to use
///
/// # Returns
/// Parsed document with text (uses vision OCR for scanned/image-heavy docs)
pub async fn parse_document_with_vision(
    path: &Path,
    client: &Client,
    vision_enabled: bool,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<ParsedDocument, String> {
    let extension = path.extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "File has no extension".to_string())?
        .to_lowercase();

    match extension.as_str() {
        "pdf" => parse_pdf_with_vision(path, client, vision_enabled, base_url, api_key, model).await,
        "docx" => parse_docx_with_vision(path, client, vision_enabled, base_url, api_key, model).await,
        "pptx" => parse_pptx_with_vision(path, client, vision_enabled, base_url, api_key, model).await,
        "txt" => parse_txt(path), // TXT doesn't need vision
        _ => Err(format!("Unsupported file type: {}", extension)),
    }
}
