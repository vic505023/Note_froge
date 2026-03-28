use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct YouTubeResult {
    pub title: String,
    pub text: String,
    pub duration_seconds: u64,
    pub channel: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// Parse YouTube video to get title, transcript, and metadata
/// Tries subtitles first, falls back to Whisper transcription if available
pub async fn parse_youtube(url: &str) -> Result<YouTubeResult, String> {
    parse_youtube_with_whisper(url, None).await
}

/// Parse YouTube video with optional Whisper fallback
pub async fn parse_youtube_with_whisper(
    url: &str,
    whisper_config: Option<WhisperConfig>,
) -> Result<YouTubeResult, String> {
    // Try direct YouTube Transcript API first (most reliable)
    let video_id = extract_video_id(url)?;

    eprintln!("Trying direct YouTube Transcript API...");
    match fetch_transcript_directly(&video_id).await {
        Ok(transcript) => {
            eprintln!("Successfully fetched transcript via direct API");
            let (title, channel, duration_seconds) = get_video_metadata(url)?;
            return Ok(YouTubeResult {
                title,
                text: transcript,
                duration_seconds,
                channel,
                url: url.to_string(),
            });
        }
        Err(e) => {
            eprintln!("Direct API failed: {}, falling back to yt-dlp", e);
        }
    }
    // Check if yt-dlp is installed
    let version_check = Command::new("yt-dlp")
        .arg("--version")
        .output();

    if version_check.is_err() {
        return Err(
            "yt-dlp not found. Install it:\n\
             Arch: sudo pacman -S yt-dlp\n\
             Ubuntu: sudo apt install yt-dlp\n\
             macOS: brew install yt-dlp"
                .to_string(),
        );
    }

    // Create temporary directory for subtitles
    let tmp_dir = std::env::temp_dir().join(format!("noteforge_yt_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Extract video ID from URL for filename
    let video_id = extract_video_id(url)?;

    // Download subtitles - simplified approach
    let mut cmd = Command::new("yt-dlp");
    cmd.arg("--write-auto-subs")
        .arg("--sub-lang")
        .arg("ru,en")
        .arg("--skip-download")
        .arg("--sub-format")
        .arg("vtt")
        .arg("--extractor-args")
        .arg("youtube:player_client=tv_embedded")  // Single client for reliability
        .arg("--cookies-from-browser").arg("firefox")
        .arg("--no-check-certificate")
        .arg("--ignore-errors");  // Don't fail on format errors

    eprintln!("Downloading subtitles with yt-dlp...");

    let output = cmd
        .arg("-o")
        .arg(tmp_dir.join(&video_id).to_string_lossy().to_string())
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    // Check for rate limiting or subtitle errors even if exit code is 0
    let stderr = String::from_utf8_lossy(&output.stderr);
    let subtitles_failed = !output.status.success()
        || stderr.contains("429")
        || stderr.contains("Too Many Requests")
        || stderr.contains("no subtitles")
        || stderr.contains("No subtitles")
        || stderr.contains("PO Token");

    // If subtitles failed and Whisper is configured, try Whisper transcription
    if subtitles_failed && whisper_config.is_some() {
        eprintln!("Subtitles unavailable, trying Whisper transcription...");
        fs::remove_dir_all(&tmp_dir).ok();

        let result = transcribe_with_whisper(url, &video_id, whisper_config.unwrap()).await;

        if let Ok(transcript) = result {
            // Get metadata
            let (title, channel, duration_seconds) = get_video_metadata(url)?;

            return Ok(YouTubeResult {
                title,
                text: transcript,
                duration_seconds,
                channel,
                url: url.to_string(),
            });
        } else {
            eprintln!("Whisper transcription also failed: {:?}", result);
        }
    }

    // If subtitles failed and no Whisper, return error
    if subtitles_failed {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err(
            "Unable to extract transcript. YouTube is blocking subtitle downloads.\n\n\
             Solutions:\n\
             1. Configure Whisper API in settings (AI → Whisper settings)\n\
             2. Update yt-dlp: sudo pacman -Syu yt-dlp (or: yt-dlp -U)\n\
             3. Wait 10-15 minutes and retry\n\n\
             Technical details:\n".to_string() + &stderr
        );
    }

    // Find downloaded .vtt file (prefer ru, then en, then any)
    let vtt_file = match find_vtt_file(&tmp_dir, &video_id) {
        Ok(file) => file,
        Err(e) => {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(format!(
                "Failed to download subtitles: {}\n\n\
                 Configure Whisper API in settings for automatic fallback transcription.",
                e
            ));
        }
    };

    // Parse VTT file
    let vtt_content = fs::read_to_string(&vtt_file)
        .map_err(|e| format!("Failed to read subtitles: {}", e))?;

    let transcript = parse_vtt(&vtt_content);

    if transcript.trim().is_empty() {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err("No subtitles available for this video".to_string());
    }

    // Get metadata (Android client for consistency)
    let metadata_output = Command::new("yt-dlp")
        .arg("--print")
        .arg("%(title)s\n%(channel)s\n%(duration)s")
        .arg("--skip-download")
        .arg("--extractor-args")
        .arg("youtube:player_client=android")
        .arg("--no-check-certificate")
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    let metadata = String::from_utf8_lossy(&metadata_output.stdout);
    let lines: Vec<&str> = metadata.trim().lines().collect();

    let title = lines.get(0).unwrap_or(&"Untitled Video").to_string();
    let channel = lines.get(1).unwrap_or(&"Unknown").to_string();
    let duration_str = lines.get(2).unwrap_or(&"0").to_string();
    let duration_seconds = duration_str.parse::<u64>().unwrap_or(0);

    // Clean up temp directory
    fs::remove_dir_all(&tmp_dir).ok();

    Ok(YouTubeResult {
        title,
        text: transcript,
        duration_seconds,
        channel,
        url: url.to_string(),
    })
}

/// Extract video ID from YouTube URL
fn extract_video_id(url: &str) -> Result<String, String> {
    // Try different URL patterns
    let patterns = [
        r"(?:youtube\.com/watch\?v=|youtu\.be/)([a-zA-Z0-9_-]{11})",
        r"youtube\.com/embed/([a-zA-Z0-9_-]{11})",
    ];

    for pattern in &patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(url) {
            if let Some(id) = caps.get(1) {
                return Ok(id.as_str().to_string());
            }
        }
    }

    Err("Invalid YouTube URL".to_string())
}

/// Find VTT subtitle file in temp directory
fn find_vtt_file(dir: &Path, _video_id: &str) -> Result<PathBuf, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read temp dir: {}", e))?;

    let mut vtt_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "vtt")
                .unwrap_or(false)
        })
        .collect();

    if vtt_files.is_empty() {
        return Err("No subtitles available for this video".to_string());
    }

    // Prefer ru, then en, then any
    let preferred_order = ["ru", "en"];
    for lang in &preferred_order {
        if let Some(file) = vtt_files.iter().find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.contains(lang))
                .unwrap_or(false)
        }) {
            return Ok(file.clone());
        }
    }

    // Return first available
    Ok(vtt_files.remove(0))
}

/// Parse VTT file and extract clean text
fn parse_vtt(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut seen_lines = HashSet::new();

    // Regex to match timestamps
    let timestamp_re = Regex::new(r"^\d{2}:\d{2}:\d{2}\.\d{3} --> \d{2}:\d{2}:\d{2}\.\d{3}").unwrap();
    // Regex to clean tags
    let tag_re = Regex::new(r"<[^>]+>").unwrap();

    let mut in_subtitle = false;

    for line in lines {
        let trimmed = line.trim();

        // Skip WEBVTT header and NOTE lines
        if trimmed.starts_with("WEBVTT") || trimmed.starts_with("NOTE") {
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            in_subtitle = false;
            continue;
        }

        // Skip timestamp lines
        if timestamp_re.is_match(trimmed) {
            in_subtitle = true;
            continue;
        }

        // Skip number-only lines (subtitle indices)
        if trimmed.chars().all(|c| c.is_numeric()) {
            continue;
        }

        // Process subtitle text
        if in_subtitle {
            // Remove tags
            let clean = tag_re.replace_all(trimmed, "");
            let clean = clean.trim();

            if !clean.is_empty() {
                // Deduplicate: YouTube subtitles often repeat lines
                if !seen_lines.contains(clean) {
                    result.push(clean.to_string());
                    seen_lines.insert(clean.to_string());
                }
            }
        }
    }

    // Join with spaces and format into paragraphs
    let text = result.join(" ");

    // Split into sentences and group into paragraphs (every 3-4 sentences)
    let sentences: Vec<&str> = text
        .split(|c| c == '.' || c == '?' || c == '!')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut paragraphs = Vec::new();
    let mut current_para = Vec::new();

    for (i, sentence) in sentences.iter().enumerate() {
        current_para.push(sentence.to_string());

        // Create paragraph every 3-4 sentences
        if (i + 1) % 4 == 0 || i == sentences.len() - 1 {
            let para = current_para.join(". ") + ".";
            paragraphs.push(para);
            current_para.clear();
        }
    }

    paragraphs.join("\n\n")
}

/// Transcribe YouTube video using Whisper API
async fn transcribe_with_whisper(
    url: &str,
    video_id: &str,
    config: WhisperConfig,
) -> Result<String, String> {
    // Download only audio (much smaller and faster)
    let tmp_dir = std::env::temp_dir().join(format!("noteforge_whisper_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let audio_path = tmp_dir.join(format!("{}.mp3", video_id));

    eprintln!("Downloading audio for Whisper transcription...");

    let output = Command::new("yt-dlp")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("5")  // Compromise between quality and size
        .arg("-o")
        .arg(audio_path.to_string_lossy().to_string())
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to download audio: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        fs::remove_dir_all(&tmp_dir).ok();
        return Err(format!("Failed to download audio: {}", stderr));
    }

    // Check if file was created
    if !audio_path.exists() {
        fs::remove_dir_all(&tmp_dir).ok();
        return Err("Audio file not created".to_string());
    }

    eprintln!("Audio downloaded, transcribing with Whisper...");

    // Send to Whisper API
    let client = Client::new();
    let file_content = fs::read(&audio_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_content)
                .file_name(format!("{}.mp3", video_id))
                .mime_str("audio/mpeg")
                .map_err(|e| format!("Failed to set MIME type: {}", e))?,
        )
        .text("model", config.model.clone());

    let response = client
        .post(format!("{}/audio/transcriptions", config.base_url))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Whisper API request failed: {}", e))?;

    // Clean up audio file
    fs::remove_dir_all(&tmp_dir).ok();

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Whisper API error {}: {}", status, error_text));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Whisper response: {}", e))?;

    let transcript = response_json["text"]
        .as_str()
        .ok_or("Invalid Whisper response format")?
        .to_string();

    eprintln!("Whisper transcription complete!");

    Ok(transcript)
}

/// Fetch transcript directly from YouTube's timedtext API (bypasses yt-dlp blocks)
async fn fetch_transcript_directly(video_id: &str) -> Result<String, String> {
    let client = Client::new();

    // Try multiple language codes
    let languages = ["ru", "en", "en-US", "en-GB"];

    for lang in &languages {
        eprintln!("Direct API: Trying language: {}", lang);

        // YouTube's timedtext API endpoint
        let url = format!(
            "https://www.youtube.com/api/timedtext?v={}&lang={}&fmt=vtt",
            video_id, lang
        );

        match client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                eprintln!("Direct API: Response status: {}", status);

                if status.is_success() {
                    match response.text().await {
                        Ok(content) => {
                            eprintln!("Direct API: Content length: {} bytes", content.len());

                            if !content.trim().is_empty() && content.contains("WEBVTT") {
                                eprintln!("✓ Direct API: Found subtitles in language: {}", lang);
                                return Ok(parse_vtt(&content));
                            } else {
                                eprintln!("Direct API: Content not valid VTT");
                            }
                        }
                        Err(e) => eprintln!("Direct API: Failed to read response: {}", e),
                    }
                } else {
                    eprintln!("Direct API: HTTP error {}", status);
                }
            }
            Err(e) => eprintln!("Direct API: Request failed: {}", e),
        }
    }

    Err("No subtitles available via direct API".to_string())
}

/// Get video metadata separately (for Whisper fallback)
fn get_video_metadata(url: &str) -> Result<(String, String, u64), String> {
    let metadata_output = Command::new("yt-dlp")
        .arg("--print")
        .arg("%(title)s\n%(channel)s\n%(duration)s")
        .arg("--skip-download")
        .arg("--extractor-args")
        .arg("youtube:player_client=android")
        .arg("--no-check-certificate")
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    if !metadata_output.status.success() {
        let stderr = String::from_utf8_lossy(&metadata_output.stderr);
        return Err(format!("Failed to get metadata: {}", stderr));
    }

    let metadata = String::from_utf8_lossy(&metadata_output.stdout);
    let lines: Vec<&str> = metadata.trim().lines().collect();

    let title = lines.get(0).unwrap_or(&"Untitled Video").to_string();
    let channel = lines.get(1).unwrap_or(&"Unknown").to_string();
    let duration_str = lines.get(2).unwrap_or(&"0").to_string();
    let duration_seconds = duration_str.parse::<u64>().unwrap_or(0);

    Ok((title, channel, duration_seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn test_parse_vtt() {
        let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:02.000
This is a test subtitle.

00:00:02.000 --> 00:00:04.000
This is a test subtitle.

00:00:04.000 --> 00:00:06.000
Second line here.
"#;
        let result = parse_vtt(vtt);
        // Should deduplicate and join
        assert!(result.contains("This is a test subtitle"));
        assert!(result.contains("Second line here"));
    }
}
