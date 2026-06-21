use crate::utils::mask_webhook_url;
use anyhow::Result;

/// Send Discord messages to all configured webhooks
pub async fn send_to_all_webhooks(
    client: &reqwest::Client,
    webhook_urls: &[String],
    username: &str,
    components: Vec<crate::models::Component>,
) -> Result<u32> {
    let mut success_count = 0;

    for webhook_url in webhook_urls {
        match crate::discord::send_discord(client, webhook_url, username, components.clone()).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                tracing::error!(
                    "Discord delivery failed for webhook {}: {:?}",
                    mask_webhook_url(webhook_url),
                    e
                );
            }
        }
    }

    Ok(success_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Component, Container, TextDisplay};

    #[test]
    fn test_build_components() {
        let components = vec![Component::Container(Container::new(
            Some(0x1E90FF),
            vec![Component::TextDisplay(TextDisplay::new("Test message"))],
        ))];

        assert_eq!(components.len(), 1);
    }

    #[tokio::test]
    async fn test_send_to_empty_webhooks() {
        let client = reqwest::Client::new();
        let webhook_urls: Vec<String> = vec![];
        let components = vec![Component::Container(Container::new(
            Some(0x1E90FF),
            vec![Component::TextDisplay(TextDisplay::new("Test"))],
        ))];

        let result = send_to_all_webhooks(&client, &webhook_urls, "Rustico", components).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
