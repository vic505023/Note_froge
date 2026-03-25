use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Get embeddings for multiple texts from OpenAI-compatible API
///
/// Batches requests if texts > 100. Retries on 429 (rate limit).
pub async fn get_embeddings(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>, String> {
    const BATCH_SIZE: usize = 100;
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    let mut all_embeddings = Vec::new();

    // Split into batches
    for batch in texts.chunks(BATCH_SIZE) {
        let request = EmbeddingRequest {
            model: model.to_string(),
            input: batch.to_vec(),
            encoding_format: Some("float".to_string()),
        };

        let mut retries = 0;
        let embeddings = loop {
            let response = client
                .post(&format!("{}/embeddings", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request)
                .send()
                .await
                .map_err(|e| format!("Embedding request failed: {}", e))?;

            if response.status().is_success() {
                let response_text = response.text().await
                    .map_err(|e| format!("Failed to read response: {}", e))?;

                eprintln!("Embeddings API response: {}", &response_text[..response_text.len().min(500)]);

                let embedding_response: EmbeddingResponse = serde_json::from_str(&response_text)
                    .map_err(|e| format!("Failed to parse embedding response: {}. Response: {}", e, &response_text[..response_text.len().min(200)]))?;

                break embedding_response.data.into_iter()
                    .map(|d| d.embedding)
                    .collect::<Vec<_>>();
            } else if response.status() == 429 && retries < MAX_RETRIES {
                // Rate limited - retry with delay
                retries += 1;
                eprintln!("Rate limited, retrying in {}ms (attempt {}/{})", RETRY_DELAY_MS, retries, MAX_RETRIES);
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                continue;
            } else {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(format!("Embedding API error {}: {}", status, error_text));
            }
        };

        all_embeddings.extend(embeddings);
    }

    Ok(all_embeddings)
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Find top-k most similar embeddings
///
/// Returns (id, source_id, chunk_text, score, page) sorted by score DESC
pub fn find_top_k(
    query_vec: &[f32],
    embeddings: &[(i64, String, String, Vec<f32>, Option<i64>)],
    k: usize,
) -> Vec<(i64, String, String, f32, Option<i64>)> {
    let mut scored: Vec<(i64, String, String, f32, Option<i64>)> = embeddings
        .iter()
        .map(|(id, source_id, chunk_text, vec, page)| {
            let score = cosine_similarity(query_vec, vec);
            (*id, source_id.clone(), chunk_text.clone(), score, *page)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    // Take top k
    scored.into_iter().take(k).collect()
}

/// Convert f32 vector to little-endian bytes for SQLite storage
pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Convert little-endian bytes to f32 vector
pub fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| {
            let bytes: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(bytes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&c, &d).abs() < 0.001);
    }

    #[test]
    fn test_vec_blob_conversion() {
        let vec = vec![1.0, 2.5, -3.7, 0.0];
        let blob = vec_to_blob(&vec);
        let restored = blob_to_vec(&blob);
        assert_eq!(vec, restored);
    }

    #[test]
    fn test_find_top_k() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![
            (1, "a".to_string(), "text a".to_string(), vec![1.0, 0.0], Some(1)),
            (2, "b".to_string(), "text b".to_string(), vec![0.5, 0.5], Some(2)),
            (3, "c".to_string(), "text c".to_string(), vec![0.0, 1.0], None),
        ];

        let top = find_top_k(&query, &embeddings, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 1); // Most similar
        assert!(top[0].3 > top[1].3); // Score descending
        assert_eq!(top[0].4, Some(1)); // Page number
    }
}
