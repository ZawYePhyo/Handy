use base64::{engine::general_purpose::STANDARD, Engine as _};
use hound::{SampleFormat, WavSpec, WavWriter};
use log::{debug, error, info};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
    code: Option<i32>,
}

/// Converts f32 audio samples (range -1.0 to 1.0) to WAV bytes
fn samples_to_wav_bytes(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

        for &sample in samples {
            // Convert f32 (-1.0 to 1.0) to i16
            let sample_i16 = (sample * i16::MAX as f32) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
    }

    Ok(cursor.into_inner())
}

/// Transcribes audio samples using Gemini API
///
/// # Arguments
/// * `api_key` - The Gemini API key
/// * `samples` - Audio samples as f32 values (normalized to -1.0 to 1.0)
/// * `language_hint` - Optional language hint for transcription
///
/// # Returns
/// The transcribed text or an error
pub async fn transcribe_audio(
    api_key: &str,
    samples: Vec<f32>,
    language_hint: Option<String>,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Gemini API key is not configured. Please add your API key in Settings.".to_string());
    }

    if samples.is_empty() {
        return Ok(String::new());
    }

    info!("Starting Gemini transcription with {} samples", samples.len());

    // Convert samples to WAV bytes (assuming 16kHz sample rate, which is what the app uses)
    let wav_bytes = samples_to_wav_bytes(&samples, 16000)?;
    debug!("Converted to WAV: {} bytes", wav_bytes.len());

    // Base64 encode the WAV data
    let base64_audio = STANDARD.encode(&wav_bytes);

    // Build the prompt
    let prompt = match language_hint {
        Some(ref lang) if lang != "auto" => {
            format!(
                "Transcribe this audio. The language is {}. Return only the transcribed text, nothing else.",
                lang
            )
        }
        _ => "Transcribe this audio. Return only the transcribed text, nothing else.".to_string(),
    };

    // Try primary model first, then fallback
    let models = ["gemini-2.5-flash", "gemini-2.0-flash"];
    let mut last_error = String::new();

    for model in &models {
        debug!("Attempting transcription with model: {}", model);

        match send_transcription_request(api_key, model, &base64_audio, &prompt).await {
            Ok(text) => {
                info!(
                    "Gemini transcription succeeded with model {}: {} chars",
                    model,
                    text.len()
                );
                return Ok(text);
            }
            Err(e) => {
                error!("Gemini transcription failed with model {}: {}", model, e);
                last_error = e;
            }
        }
    }

    Err(format!("Gemini transcription failed: {}", last_error))
}

async fn send_transcription_request(
    api_key: &str,
    model: &str,
    base64_audio: &str,
    prompt: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![
                Part::Text {
                    text: prompt.to_string(),
                },
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: "audio/wav".to_string(),
                        data: base64_audio.to_string(),
                    },
                },
            ],
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!("API request failed with status {}: {}", status, response_text));
    }

    let gemini_response: GeminiResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {} - Response: {}", e, response_text))?;

    if let Some(error) = gemini_response.error {
        return Err(format!(
            "Gemini API error (code {:?}): {}",
            error.code, error.message
        ));
    }

    // Extract text from the response
    let text = gemini_response
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(text)
}
