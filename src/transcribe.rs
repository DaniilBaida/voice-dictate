use async_openai::{
    config::OpenAIConfig,
    types::{AudioInput, CreateTranscriptionRequestArgs},
    Client,
};

pub struct Transcriber {
    client: Client<OpenAIConfig>,
    model: String,
    language: Option<String>,
    prompt: Option<String>,
}

impl Transcriber {
    pub fn new(api_key: &str, api_base: &str, model: &str, language: &str, prompt: &str) -> Self {
        let cfg = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base);
        Self {
            client: Client::with_config(cfg),
            model: model.to_string(),
            language: if language.is_empty() { None } else { Some(language.to_string()) },
            prompt: if prompt.is_empty() { None } else { Some(prompt.to_string()) },
        }
    }

    pub async fn transcribe(&self, wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        let file = AudioInput::from_vec_u8("audio.wav".to_string(), wav_bytes);

        let mut req = CreateTranscriptionRequestArgs::default();
        req.file(file);
        req.model(self.model.clone());
        // response_format left as default (Json) so transcribe() deserialises correctly

        if let Some(lang) = &self.language {
            req.language(lang.clone());
        }
        if let Some(p) = &self.prompt {
            req.prompt(p.clone());
        }

        let request = req.build()?;
        let response = self.client.audio().transcribe(request).await?;
        Ok(response.text.trim().to_string())
    }
}
