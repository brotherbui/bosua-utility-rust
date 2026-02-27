//! OCR command supporting multiple AI providers.
//!
//! Provides the `ocr` command for extracting text from images using
//! Anthropic, Gemini, or OpenAI as the OCR provider.

use std::path::Path;

use base64::Engine;
use clap::{Arg, ArgMatches, Command};
use serde_json::json;

use crate::cli::{CommandBuilder, CommandCategory, CommandMeta};
use crate::config::dynamic::DynamicConfig;
use crate::errors::{BosuaError, Result};
use crate::http_client::HttpClient;

/// Build the `ocr` clap command.
pub fn ocr_command() -> Command {
    Command::new("ocr")
        .about("OCR text extraction from images")
        .arg(
            Arg::new("image")
                .required(true)
                .help("Input image file"),
        )
        .arg(
            Arg::new("provider")
                .long("provider")
                .short('p')
                .value_parser(["anthropic", "gemini", "openai"])
                .default_value("anthropic")
                .help("OCR provider (anthropic, gemini, openai)"),
        )
        .arg(
            Arg::new("api-key")
                .long("api-key")
                .short('k')
                .help("API key for the selected provider"),
        )
}

/// Build the `CommandMeta` for registry registration.
pub fn ocr_meta() -> CommandMeta {
    CommandBuilder::from_clap(ocr_command())
        .category(CommandCategory::Developer)
        .build()
}

/// Supported OCR providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrProvider {
    Anthropic,
    Gemini,
    OpenAI,
}

impl OcrProvider {
    /// Parse a provider name string.
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            "openai" => Ok(Self::OpenAI),
            _ => Err(BosuaError::Command(format!(
                "Unknown OCR provider: {}. Use anthropic, gemini, or openai.",
                s
            ))),
        }
    }

    /// Environment variable name for the API key.
    pub fn env_var_name(&self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
            Self::OpenAI => "OPENAI_API_KEY",
        }
    }

    /// Display name for the provider.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Gemini",
            Self::OpenAI => "OpenAI",
        }
    }

    /// Send an image to the AI provider for OCR text extraction.
    pub async fn extract_text(
        &self,
        http: &HttpClient,
        api_key: &str,
        image_base64: &str,
        mime_type: &str,
    ) -> Result<String> {
        let client = http.get_client().await;

        match self {
            Self::Anthropic => {
                let body = json!({
                    "model": "claude-sonnet-4-20250514",
                    "max_tokens": 4096,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime_type,
                                    "data": image_base64
                                }
                            },
                            {
                                "type": "text",
                                "text": "Extract all text from this image. Return only the extracted text, preserving the original formatting as much as possible."
                            }
                        ]
                    }]
                });

                let resp = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| BosuaError::Cloud {
                        service: "Anthropic".into(),
                        message: e.to_string(),
                    })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(BosuaError::Cloud {
                        service: "Anthropic".into(),
                        message: format!("API error ({}): {}", status, text),
                    });
                }

                let json: serde_json::Value = resp.json().await.map_err(|e| {
                    BosuaError::Cloud {
                        service: "Anthropic".into(),
                        message: format!("Failed to parse response: {}", e),
                    }
                })?;

                json["content"][0]["text"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| BosuaError::Cloud {
                        service: "Anthropic".into(),
                        message: "Unexpected response format".into(),
                    })
            }
            Self::Gemini => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                    api_key
                );

                let body = json!({
                    "contents": [{
                        "parts": [
                            {
                                "inline_data": {
                                    "mime_type": mime_type,
                                    "data": image_base64
                                }
                            },
                            {
                                "text": "Extract all text from this image. Return only the extracted text, preserving the original formatting as much as possible."
                            }
                        ]
                    }]
                });

                let resp = client
                    .post(&url)
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| BosuaError::Cloud {
                        service: "Gemini".into(),
                        message: e.to_string(),
                    })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(BosuaError::Cloud {
                        service: "Gemini".into(),
                        message: format!("API error ({}): {}", status, text),
                    });
                }

                let json: serde_json::Value = resp.json().await.map_err(|e| {
                    BosuaError::Cloud {
                        service: "Gemini".into(),
                        message: format!("Failed to parse response: {}", e),
                    }
                })?;

                json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| BosuaError::Cloud {
                        service: "Gemini".into(),
                        message: "Unexpected response format".into(),
                    })
            }
            Self::OpenAI => {
                let body = json!({
                    "model": "gpt-4o",
                    "messages": [{
                        "role": "user",
                        "content": [
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:{};base64,{}", mime_type, image_base64)
                                }
                            },
                            {
                                "type": "text",
                                "text": "Extract all text from this image. Return only the extracted text, preserving the original formatting as much as possible."
                            }
                        ]
                    }],
                    "max_tokens": 4096
                });

                let resp = client
                    .post("https://api.openai.com/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("content-type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| BosuaError::Cloud {
                        service: "OpenAI".into(),
                        message: e.to_string(),
                    })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(BosuaError::Cloud {
                        service: "OpenAI".into(),
                        message: format!("API error ({}): {}", status, text),
                    });
                }

                let json: serde_json::Value = resp.json().await.map_err(|e| {
                    BosuaError::Cloud {
                        service: "OpenAI".into(),
                        message: format!("Failed to parse response: {}", e),
                    }
                })?;

                json["choices"][0]["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| BosuaError::Cloud {
                        service: "OpenAI".into(),
                        message: "Unexpected response format".into(),
                    })
            }
        }
    }
}

/// Detect MIME type from file extension.
fn detect_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("tiff" | "tif") => "image/tiff",
        _ => "image/png",
    }
}

/// Resolve the API key from CLI flag, environment variable, or config.
///
/// Priority: CLI flag > environment variable > error
pub fn resolve_api_key(
    cli_key: Option<&String>,
    provider: &OcrProvider,
    _config: &DynamicConfig,
) -> Result<String> {
    if let Some(key) = cli_key {
        return Ok(key.clone());
    }

    if let Ok(key) = std::env::var(provider.env_var_name()) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    Err(BosuaError::Auth(format!(
        "{} API key not found. Provide it via --api-key or set the {} environment variable.",
        provider.name(),
        provider.env_var_name()
    )))
}

/// Handle the `ocr` command.
pub async fn handle_ocr(
    matches: &ArgMatches,
    config: &DynamicConfig,
    http: &HttpClient,
) -> Result<()> {
    let image = matches.get_one::<String>("image").unwrap();
    let image_path = Path::new(image);

    if !image_path.exists() {
        return Err(BosuaError::Command(format!(
            "Image file not found: {}",
            image
        )));
    }

    let provider_str = matches
        .get_one::<String>("provider")
        .map(|s| s.as_str())
        .unwrap_or("anthropic");
    let provider = OcrProvider::from_str(provider_str)?;

    let api_key = resolve_api_key(
        matches.get_one::<String>("api-key"),
        &provider,
        config,
    )?;

    let image_data = std::fs::read(image_path).map_err(|e| {
        BosuaError::Command(format!("Failed to read image file '{}': {}", image, e))
    })?;

    let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data);
    let mime_type = detect_mime_type(image_path);

    println!("Extracting text using {} ...", provider.name());
    let text = provider
        .extract_text(http, &api_key, &image_base64, mime_type)
        .await?;

    println!("\n{}", text);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_command_parses_image() {
        let cmd = ocr_command();
        let matches = cmd.try_get_matches_from(["ocr", "photo.png"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("image").map(|s| s.as_str()),
            Some("photo.png"),
        );
    }

    #[test]
    fn test_ocr_default_provider() {
        let cmd = ocr_command();
        let matches = cmd.try_get_matches_from(["ocr", "photo.png"]).unwrap();
        assert_eq!(
            matches.get_one::<String>("provider").map(|s| s.as_str()),
            Some("anthropic"),
        );
    }

    #[test]
    fn test_ocr_with_gemini_provider() {
        let cmd = ocr_command();
        let matches = cmd
            .try_get_matches_from(["ocr", "photo.png", "--provider", "gemini"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("provider").map(|s| s.as_str()),
            Some("gemini"),
        );
    }

    #[test]
    fn test_ocr_with_openai_provider() {
        let cmd = ocr_command();
        let matches = cmd
            .try_get_matches_from(["ocr", "photo.png", "--provider", "openai"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("provider").map(|s| s.as_str()),
            Some("openai"),
        );
    }

    #[test]
    fn test_ocr_invalid_provider_rejected() {
        let cmd = ocr_command();
        let result = cmd.try_get_matches_from(["ocr", "photo.png", "--provider", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ocr_with_api_key() {
        let cmd = ocr_command();
        let matches = cmd
            .try_get_matches_from(["ocr", "photo.png", "--api-key", "sk-test123"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("api-key").map(|s| s.as_str()),
            Some("sk-test123"),
        );
    }

    #[test]
    fn test_ocr_requires_image() {
        let cmd = ocr_command();
        let result = cmd.try_get_matches_from(["ocr"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ocr_meta() {
        let meta = ocr_meta();
        assert_eq!(meta.name, "ocr");
        assert_eq!(meta.category, CommandCategory::Developer);
        assert!(!meta.description.is_empty());
    }

    #[test]
    fn test_ocr_provider_from_str() {
        assert_eq!(OcrProvider::from_str("anthropic").unwrap(), OcrProvider::Anthropic);
        assert_eq!(OcrProvider::from_str("gemini").unwrap(), OcrProvider::Gemini);
        assert_eq!(OcrProvider::from_str("openai").unwrap(), OcrProvider::OpenAI);
        assert!(OcrProvider::from_str("invalid").is_err());
    }

    #[test]
    fn test_ocr_provider_env_var_names() {
        assert_eq!(OcrProvider::Anthropic.env_var_name(), "ANTHROPIC_API_KEY");
        assert_eq!(OcrProvider::Gemini.env_var_name(), "GEMINI_API_KEY");
        assert_eq!(OcrProvider::OpenAI.env_var_name(), "OPENAI_API_KEY");
    }

    #[test]
    fn test_detect_mime_type() {
        assert_eq!(detect_mime_type(Path::new("photo.png")), "image/png");
        assert_eq!(detect_mime_type(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(detect_mime_type(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(detect_mime_type(Path::new("photo.gif")), "image/gif");
        assert_eq!(detect_mime_type(Path::new("photo.webp")), "image/webp");
        assert_eq!(detect_mime_type(Path::new("photo.bmp")), "image/bmp");
        assert_eq!(detect_mime_type(Path::new("photo.tiff")), "image/tiff");
        assert_eq!(detect_mime_type(Path::new("photo.unknown")), "image/png");
    }

    #[test]
    fn test_resolve_api_key_from_cli() {
        let config = DynamicConfig::default();
        let key = "sk-test-key".to_string();
        let result = resolve_api_key(Some(&key), &OcrProvider::Anthropic, &config);
        assert_eq!(result.unwrap(), "sk-test-key");
    }

    #[test]
    fn test_resolve_api_key_missing() {
        let config = DynamicConfig::default();
        // Ensure env var is not set
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = resolve_api_key(None, &OcrProvider::Anthropic, &config);
        assert!(result.is_err());
        match result.unwrap_err() {
            BosuaError::Auth(msg) => {
                assert!(msg.contains("Anthropic"));
                assert!(msg.contains("ANTHROPIC_API_KEY"));
            }
            e => panic!("Expected BosuaError::Auth, got {:?}", e),
        }
    }
}
