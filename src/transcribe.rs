use reqwest::multipart::{Form, Part};
use serde::Deserialize;

pub struct Transcriber {
    client: reqwest::Client,
    server_url: String,
    model: String,
    language: Option<String>,
    automatic_punctuation: bool,
}

impl Transcriber {
    pub fn new(server_url: &str, model: &str, language: &str, automatic_punctuation: bool) -> Self {
        Self {
            client: reqwest::Client::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            language: if language.is_empty() {
                None
            } else {
                Some(language.to_string())
            },
            automatic_punctuation,
        }
    }

    pub async fn transcribe(&self, wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        let audio = Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let mut form = Form::new()
            .part("file", audio)
            .text("model", self.model.clone())
            .text(
                "automatic_punctuation",
                self.automatic_punctuation.to_string(),
            );

        if let Some(lang) = &self.language {
            form = form.text("language", lang.clone());
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.server_url))
            .multipart(form)
            .send()
            .await?
            .error_for_status()?
            .json::<TranscriptionResponse>()
            .await?;
        Ok(response.text.trim().to_string())
    }
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}
