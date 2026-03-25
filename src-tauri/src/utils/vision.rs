use base64::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct VisionMessage {
    role: String,
    content: Vec<VisionContent>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum VisionContent {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
    detail: String,
}

#[derive(Serialize)]
struct VisionRequest {
    model: String,
    messages: Vec<VisionMessage>,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct VisionResponse {
    choices: Vec<VisionChoice>,
}

#[derive(Deserialize)]
struct VisionChoice {
    message: VisionResponseMessage,
}

#[derive(Deserialize)]
struct VisionResponseMessage {
    content: String,
}

/// Describe image content using Vision API (OpenAI-compatible)
///
/// Extracts text and describes diagrams, graphs, schemas, formulas, tables
/// from document pages and presentation slides.
///
/// # Arguments
/// * `client` - HTTP client
/// * `base_url` - API base URL (e.g., "https://api.ranvik.ru/v1")
/// * `api_key` - API key for authentication
/// * `model` - Model to use (e.g., "gpt-4o-mini")
/// * `image_path` - Path to image file (PNG, JPG, etc.)
/// * `context_hint` - Context about the image (e.g., "Page 3 of physics textbook")
///
/// # Returns
/// Description of image content with all visible text and visual elements
pub async fn describe_image(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    image_path: &Path,
    context_hint: &str,
) -> Result<String, String> {
    // Read image file
    let image_bytes = fs::read(image_path)
        .map_err(|e| format!("Failed to read image {}: {}", image_path.display(), e))?;

    // Encode to base64
    let base64_data = BASE64_STANDARD.encode(&image_bytes);

    // Determine image format from extension
    let extension = image_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let mime_type = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png", // default
    };

    // Build request
    let system_msg = VisionMessage {
        role: "system".to_string(),
        content: vec![VisionContent::Text {
            text: "You extract and describe content from document pages and presentation slides. Output ALL text you see (including handwritten). Describe any diagrams, graphs, schemas, formulas, tables. Use the original language of the document. Be thorough but structured.".to_string(),
        }],
    };

    let user_msg = VisionMessage {
        role: "user".to_string(),
        content: vec![
            VisionContent::Text {
                text: format!("Describe everything on this page. Context: {}", context_hint),
            },
            VisionContent::ImageUrl {
                image_url: ImageUrl {
                    url: format!("data:{};base64,{}", mime_type, base64_data),
                    detail: "high".to_string(),
                },
            },
        ],
    };

    let request = VisionRequest {
        model: model.to_string(),
        messages: vec![system_msg, user_msg],
        max_tokens: 2000,
    };

    // Send request
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    eprintln!("Vision API request to: {} (image size: {} bytes)", url, image_bytes.len());

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Vision API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Vision API error {}: {}", status, error_text));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let vision_response: VisionResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse vision response: {}. Response: {}", e, &response_text[..response_text.len().min(200)]))?;

    let content = vision_response
        .choices
        .first()
        .ok_or("No choices in vision response")?
        .message
        .content
        .clone();

    eprintln!("Vision OCR extracted {} chars", content.len());

    Ok(content)
}
