//! Discord webhook notifications for the anti-AFK (started, stopped, rejoined, errors...).

use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Started,
    Stopped,
    Action,
    Disconnected,
    Rejoined,
    Error,
    Test,
}

impl Event {
    pub fn setting(self) -> &'static str {
        match self {
            Event::Started => "WebhookOnStart",
            Event::Stopped => "WebhookOnStop",
            Event::Action => "WebhookOnAction",
            Event::Disconnected | Event::Rejoined => "WebhookOnRejoin",
            Event::Error => "WebhookOnError",
            Event::Test => "",
        }
    }

    /// In English, translated before sending
    pub fn title(self) -> &'static str {
        match self {
            Event::Started => crate::text!("Anti-AFK started"),
            Event::Stopped => crate::text!("Anti-AFK stopped"),
            Event::Action => crate::text!("Anti-AFK cycle done"),
            Event::Disconnected => crate::text!("Account disconnected"),
            Event::Rejoined => crate::text!("Account rejoined"),
            Event::Error => crate::text!("Anti-AFK error"),
            Event::Test => crate::text!("Test message"),
        }
    }

    fn color(self) -> u32 {
        match self {
            Event::Started | Event::Rejoined => 0x22C55E,
            Event::Stopped => 0x94A3B8,
            Event::Action | Event::Test => 0x4F7CFF,
            Event::Disconnected => 0xF59E0B,
            Event::Error => 0xEF4444,
        }
    }
}

/// Only real Discord webhook links are accepted, so nothing is ever posted elsewhere by mistake
pub fn is_webhook(url: &str) -> bool {
    let url = url.trim();
    ["https://discord.com/api/webhooks/", "https://discordapp.com/api/webhooks/", "https://ptb.discord.com/api/webhooks/", "https://canary.discord.com/api/webhooks/"].iter().any(|p| url.starts_with(p))
}

/// `title` and `text` are already in the app's language
pub async fn send(http: &reqwest::Client, url: &str, event: Event, title: &str, text: &str) -> anyhow::Result<()> {
    if !is_webhook(url) {
        anyhow::bail!("That isn't a Discord webhook link");
    }

    let body = json!({
        "username": "Roblox Account Manager",
        "embeds": [{
            "title": title,
            "description": text,
            "color": event.color(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "footer": { "text": "Roblox Account Manager · Anti-AFK" },
        }],
    });

    let response = http.post(url.trim()).json(&body).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("Discord answered {}", response.status());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_discord_webhooks() {
        assert!(is_webhook("https://discord.com/api/webhooks/1/abc"));
        assert!(!is_webhook("https://example.com/api/webhooks/1/abc"));
        assert!(!is_webhook("http://discord.com/api/webhooks/1/abc"));
    }
}
