//! Telegram bot integration for sending messages and notifications.
//!
//! Uses the Telegram Bot API via HTTP (reqwest) to send messages and
//! notifications to configured chat IDs.

use serde::{Deserialize, Serialize};
use tracing;

use crate::errors::{BosuaError, Result};

/// Configuration for the Telegram bot.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot API token from BotFather.
    pub bot_token: String,
    /// Default chat ID to send messages to.
    pub chat_id: String,
}

/// Response from the Telegram Bot API.
#[derive(Debug, Deserialize)]
pub struct TelegramResponse {
    /// Whether the request was successful.
    pub ok: bool,
    /// Optional description on failure.
    pub description: Option<String>,
}

/// Payload for the `sendMessage` API call.
#[derive(Debug, Serialize)]
struct SendMessagePayload {
    chat_id: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<String>,
}

/// Telegram bot client for sending messages and notifications.
#[derive(Debug, Clone)]
pub struct TelegramBot {
    config: TelegramConfig,
    client: reqwest::Client,
}

impl TelegramBot {
    /// Create a new Telegram bot with the given configuration.
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Build the base URL for a Telegram Bot API method.
    fn api_url(&self, method: &str) -> String {
        format!(
            "https://api.telegram.org/bot{}/{}",
            self.config.bot_token, method
        )
    }

    /// Send a text message to the default chat.
    pub async fn send_message(&self, text: &str) -> Result<()> {
        self.send_message_to(&self.config.chat_id, text).await
    }

    /// Send a text message to a specific chat ID.
    pub async fn send_message_to(&self, chat_id: &str, text: &str) -> Result<()> {
        let payload = SendMessagePayload {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: None,
        };

        tracing::debug!(chat_id, text_len = text.len(), "sending telegram message");

        let resp = self
            .client
            .post(&self.api_url("sendMessage"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "telegram".into(),
                message: format!("failed to send message: {}", e),
            })?;

        let tg_resp: TelegramResponse = resp.json().await.map_err(|e| BosuaError::Cloud {
            service: "telegram".into(),
            message: format!("failed to parse response: {}", e),
        })?;

        if !tg_resp.ok {
            return Err(BosuaError::Cloud {
                service: "telegram".into(),
                message: tg_resp
                    .description
                    .unwrap_or_else(|| "unknown error".into()),
            });
        }

        Ok(())
    }

    /// Send a notification with a title and body.
    ///
    /// Formats the message as bold title + body using Telegram HTML parse mode.
    pub async fn send_notification(&self, title: &str, body: &str) -> Result<()> {
        let payload = SendMessagePayload {
            chat_id: self.config.chat_id.clone(),
            text: format!("<b>{}</b>\n{}", title, body),
            parse_mode: Some("HTML".into()),
        };

        tracing::debug!(title, "sending telegram notification");

        let resp = self
            .client
            .post(&self.api_url("sendMessage"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| BosuaError::Cloud {
                service: "telegram".into(),
                message: format!("failed to send notification: {}", e),
            })?;

        let tg_resp: TelegramResponse = resp.json().await.map_err(|e| BosuaError::Cloud {
            service: "telegram".into(),
            message: format!("failed to parse response: {}", e),
        })?;

        if !tg_resp.ok {
            return Err(BosuaError::Cloud {
                service: "telegram".into(),
                message: tg_resp
                    .description
                    .unwrap_or_else(|| "unknown error".into()),
            });
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TelegramConfig {
        TelegramConfig {
            bot_token: "123456:ABC-DEF".into(),
            chat_id: "987654321".into(),
        }
    }

    #[test]
    fn telegram_bot_creation() {
        let bot = TelegramBot::new(test_config());
        assert_eq!(bot.config.bot_token, "123456:ABC-DEF");
        assert_eq!(bot.config.chat_id, "987654321");
    }

    #[test]
    fn api_url_construction() {
        let bot = TelegramBot::new(test_config());
        let url = bot.api_url("sendMessage");
        assert_eq!(url, "https://api.telegram.org/bot123456:ABC-DEF/sendMessage");
    }

    #[test]
    fn api_url_get_me() {
        let bot = TelegramBot::new(test_config());
        let url = bot.api_url("getMe");
        assert_eq!(url, "https://api.telegram.org/bot123456:ABC-DEF/getMe");
    }

    #[test]
    fn send_message_payload_serialization() {
        let payload = SendMessagePayload {
            chat_id: "123".into(),
            text: "hello".into(),
            parse_mode: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["chat_id"], "123");
        assert_eq!(json["text"], "hello");
        assert!(json.get("parse_mode").is_none());
    }

    #[test]
    fn send_message_payload_with_parse_mode() {
        let payload = SendMessagePayload {
            chat_id: "123".into(),
            text: "<b>bold</b>".into(),
            parse_mode: Some("HTML".into()),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["parse_mode"], "HTML");
    }

    #[test]
    fn telegram_response_ok() {
        let json = r#"{"ok": true}"#;
        let resp: TelegramResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert!(resp.description.is_none());
    }

    #[test]
    fn telegram_response_error() {
        let json = r#"{"ok": false, "description": "Unauthorized"}"#;
        let resp: TelegramResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.description.as_deref(), Some("Unauthorized"));
    }
}
