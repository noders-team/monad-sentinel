use super::Notifier;
use crate::config::Secrets;

pub struct Telegram {
    token: String,
    chat_id: String,
}

impl Telegram {
    pub fn new(secrets: &Secrets) -> Self {
        Self {
            token: secrets.telegram_token.clone(),
            chat_id: secrets.telegram_chat_id.clone(),
        }
    }
}

impl Notifier for Telegram {
    fn send(&self, text: &str) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        crate::notify::retry_with_backoff(3, 200, || {
            ureq::post(&url)
                .send_form(&[("chat_id", self.chat_id.as_str()), ("text", text)])
                .map_err(|e| match e {
                    ureq::Error::Status(code, _) => {
                        anyhow::anyhow!("telegram sendMessage failed: HTTP {code}")
                    }
                    ureq::Error::Transport(_) => {
                        anyhow::anyhow!("telegram sendMessage failed: transport error")
                    }
                })?;
            Ok(())
        })
    }
}
