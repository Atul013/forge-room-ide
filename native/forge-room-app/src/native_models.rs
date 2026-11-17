//! Text-only native HTTP inference. No filesystem, command or model-requested tools execute here.
use crate::agent::Event;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod panel;
#[cfg(test)]
pub(crate) mod tests;
const MAX_INPUT: usize = 512 * 1024;
const MAX_OUTPUT: usize = 512 * 1024;
const MAX_WIRE: usize = 4 * 1024 * 1024;
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    #[default]
    Chat,
    Messages,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub key_env: String,
    pub protocol: Protocol,
    pub max_tokens: u32,
    pub seconds: u64,
    pub modern_limit: bool,
    pub history: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434/v1".into(),
            model: String::new(),
            key_env: String::new(),
            protocol: Protocol::Chat,
            max_tokens: 4096,
            seconds: 120,
            modern_limit: false,
            history: false,
        }
    }
}
impl Config {
    pub fn url(&self, suffix: &str) -> Result<reqwest::Url, String> {
        if self.base_url.len() > 2048 {
            return Err("Endpoint is too long".into());
        }
        let mut url = reqwest::Url::parse(self.base_url.trim())
            .map_err(|_| "Use a full HTTP(S) API base URL")?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(
                "Endpoint must not contain credentials, query parameters or fragments".into(),
            );
        }
        let host = url.host_str().ok_or("Endpoint needs a host")?;
        let loopback = host.eq_ignore_ascii_case("localhost")
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback());
        if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
            return Err("Use HTTPS, or HTTP on a literal loopback/localhost server".into());
        }
        if url.scheme() == "http" && host.eq_ignore_ascii_case("localhost") {
            url.set_host(Some("127.0.0.1"))
                .map_err(|_| "Invalid localhost endpoint")?;
        }
        let path = format!("{}/{}", url.path().trim_end_matches('/'), suffix);
        url.set_path(&path);
        Ok(url)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.url("models")?;
        if !self.key_env.is_empty()
            && (self.key_env.len() > 100
                || !self.key_env.bytes().enumerate().all(|(i, b)| {
                    b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit())
                }))
        {
            return Err("Enter an environment variable name, not an API key".into());
        }
        if !(128..=32768).contains(&self.max_tokens) || !(10..=600).contains(&self.seconds) {
            return Err("Output limit must be 128–32768 tokens; time limit 10–600 seconds".into());
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct Prepared {
    pub config: Config,
    pub body: Value,
    pub prompt: String,
}
pub fn prepare(config: &Config, prompt: String, history: &[Value]) -> Result<Prepared, String> {
    config.validate()?;
    if config.model.trim().is_empty()
        || config.model.len() > 200
        || config.model.chars().any(char::is_control)
    {
        return Err("Choose a nonempty model ID under 200 characters".into());
    }
    let mut messages = if config.history {
        history.to_vec()
    } else {
        Vec::new()
    };
    if messages.len() > 16 {
        return Err("Conversation is full; start a new native conversation".into());
    }
    messages.push(json!({"role":"user","content":prompt}));
    let mut body = json!({"model":config.model,"messages":messages,"stream":true});
    let limit = if config.protocol == Protocol::Chat && config.modern_limit {
        "max_completion_tokens"
    } else {
        "max_tokens"
    };
    body[limit] = json!(config.max_tokens);
    if serde_json::to_vec(&body)
        .map_err(|_| "Could not build request")?
        .len()
        > MAX_INPUT
    {
        return Err(
            "Native request exceeds 512 KiB; remove context or start a new native conversation"
                .into(),
        );
    }
    Ok(Prepared {
        config: config.clone(),
        body,
        prompt,
    })
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| "Could not initialize HTTPS client".into())
}
fn request(
    client: &reqwest::Client,
    config: &Config,
    suffix: &str,
    post: bool,
) -> Result<reqwest::RequestBuilder, String> {
    let url = config.url(suffix)?;
    let mut request = if post {
        client.post(url)
    } else {
        client.get(url)
    };
    if config.protocol == Protocol::Messages {
        request = request.header("anthropic-version", "2023-06-01");
    }
    if !config.key_env.is_empty() {
        let key = std::env::var(&config.key_env).map_err(
            |_| "API key environment variable is missing; set it before starting Forge Room",
        )?;
        if key.trim().is_empty() || key.len() > 8192 {
            return Err("API key environment variable is empty or too long".into());
        }
        let mut value =
            reqwest::header::HeaderValue::from_str(&if config.protocol == Protocol::Messages {
                key
            } else {
                format!("Bearer {key}")
            })
            .map_err(|_| "Invalid API key environment value")?;
        value.set_sensitive(true);
        request = if config.protocol == Protocol::Messages {
            request.header("x-api-key", value)
        } else {
            request.header(reqwest::header::AUTHORIZATION, value)
        };
    }
    Ok(request)
}
#[derive(Default)]
struct Mail {
    text: String,
    done: Option<Result<(), String>>,
    consumed: usize,
}
pub struct Run {
    mail: Arc<Mutex<Mail>>,
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Run {
    pub fn poll(&mut self) -> Vec<Event> {
        let mut mail = self.mail.lock().unwrap();
        let mut events = Vec::new();
        if mail.consumed < mail.text.len() {
            events.push(Event::Text(mail.text[mail.consumed..].into()));
            mail.consumed = mail.text.len();
        }
        if let Some(done) = mail.done.take() {
            events.push(Event::Done {
                cost: None,
                error: done.err(),
            });
        }
        events
    }
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        self.cancel();
    }
}
pub fn start(prepared: Prepared, wake: impl Fn() + Send + 'static) -> Result<Run, String> {
    prepared.config.validate()?;
    // Authentication validation happens before a worker/request is created; secrets are never retained in config.
    let _ = request(&client()?, &prepared.config, "models", false)?;
    let mail = Arc::new(Mutex::new(Mail::default()));
    let worker = mail.clone();
    let (cancel, canceled) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "Could not start native inference worker")?;
            let outcome = runtime.block_on(async {
                tokio::select! {
                    _=canceled=>Err("Stopped".into()),
                    _=tokio::time::sleep(Duration::from_secs(prepared.config.seconds))=>Err("Native request reached its time limit".into()),
                    result=infer(prepared,&worker,&wake)=>result,
                }
            });
            // Resolver work can be an OS blocking call. Do not make Stop/timeouts wait on it.
            runtime.shutdown_background();
            outcome
        })();
        worker.lock().unwrap().done = Some(result);
        wake();
    });
    Ok(Run {
        mail,
        cancel: Some(cancel),
    })
}
async fn infer(
    prepared: Prepared,
    mail: &Arc<Mutex<Mail>>,
    wake: &impl Fn(),
) -> Result<(), String> {
    let suffix = if prepared.config.protocol == Protocol::Chat {
        "chat/completions"
    } else {
        "messages"
    };
    let response = request(&client()?, &prepared.config, suffix, true)?
        .json(&prepared.body)
        .send()
        .await
        .map_err(
            |_| "Native connection failed; check server address, certificate and connectivity",
        )?;
    if !response.status().is_success() {
        return Err(format!(
            "Native provider returned HTTP {}; response body omitted",
            response.status().as_u16()
        ));
    }
    let streaming = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("text/event-stream"))
        });
    let mut chunks = response.bytes_stream();
    let mut wire = 0;
    let mut decoder = Decoder::default();
    let mut json_bytes = Vec::new();
    while let Some(bytes) = chunks.next().await {
        let bytes = bytes.map_err(|_| "Native response connection was interrupted")?;
        wire += bytes.len();
        if wire > MAX_WIRE {
            return Err("Native response exceeded the 4 MiB transport limit".into());
        }
        if streaming {
            for data in decoder.feed(&bytes)? {
                if data == "[DONE]" {
                    decoder.ended = true;
                    continue;
                }
                let value: Value = serde_json::from_str(&data)
                    .map_err(|_| "Native server sent invalid streaming JSON")?;
                let text = decode_event(prepared.config.protocol, &value, &mut decoder)?;
                if !text.is_empty() {
                    append(mail, &text)?;
                    wake();
                }
            }
            if decoder.ended {
                break;
            }
        } else {
            json_bytes.extend_from_slice(&bytes);
        }
    }
    if streaming {
        if !decoder.ended || !decoder.complete {
            return Err("Native stream ended without a complete answer; partial output was not accepted for edits".into());
        }
    } else {
        let value: Value =
            serde_json::from_slice(&json_bytes).map_err(|_| "Native server sent invalid JSON")?;
        let text = decode_full(prepared.config.protocol, &value)?;
        append(mail, &text)?;
        wake();
    }
    if mail.lock().unwrap().text.trim().is_empty() {
        return Err("Native model returned no text".into());
    }
    Ok(())
}
fn append(mail: &Arc<Mutex<Mail>>, text: &str) -> Result<(), String> {
    let mut mail = mail.lock().unwrap();
    if mail.text.len() + text.len() > MAX_OUTPUT {
        return Err("Native text output exceeded 512 KiB".into());
    }
    mail.text.push_str(&crate::agent::sanitize(text));
    Ok(())
}
#[derive(Default)]
struct Decoder {
    pending: Vec<u8>,
    data: String,
    ended: bool,
    complete: bool,
}
impl Decoder {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        self.pending.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            if end > 256 * 1024 {
                return Err("Native streaming line is too large".into());
            }
            let bytes: Vec<_> = self.pending.drain(..=end).collect();
            let line = std::str::from_utf8(&bytes[..end])
                .map_err(|_| "Native stream is not UTF-8")?
                .trim_end_matches('\r');
            if line.is_empty() {
                if !self.data.is_empty() {
                    events.push(std::mem::take(&mut self.data));
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
            }
            if self.data.len() > 256 * 1024 {
                return Err("Native streaming event is too large".into());
            }
        }
        if self.pending.len() > 256 * 1024 {
            return Err("Native streaming line is too large".into());
        }
        Ok(events)
    }
}
fn reason(reason: Option<&str>) -> Result<bool, String> {
    match reason {
        None => Ok(false),
        Some("stop" | "end_turn" | "stop_sequence") => Ok(true),
        Some("length" | "max_tokens") => Err(
            "Model reached its output token limit; partial output was not accepted for edits"
                .into(),
        ),
        Some(_) => {
            Err("Model stopped with an unsupported reason; tools/refusals are not executed".into())
        }
    }
}
fn tools(value: &Value) -> bool {
    ["tool_calls", "function_call"].iter().any(|key| {
        value
            .get(*key)
            .is_some_and(|v| !v.is_null() && !v.as_array().is_some_and(Vec::is_empty))
    })
}
fn decode_event(protocol: Protocol, v: &Value, decoder: &mut Decoder) -> Result<String, String> {
    if v.get("error").is_some() || v["type"].as_str() == Some("error") {
        return Err("Native provider reported an error; response details omitted".into());
    }
    if protocol == Protocol::Chat {
        let Some(choice) = v["choices"].as_array().and_then(|a| a.first()) else {
            return Ok(String::new());
        };
        if tools(&choice["delta"]) {
            return Err("Native tools are disabled; no requested tool was executed".into());
        }
        if choice["delta"].get("refusal").is_some_and(|v| !v.is_null()) {
            return Err("Native provider refused the request".into());
        }
        decoder.complete |= reason(choice["finish_reason"].as_str())?;
        Ok(choice["delta"]["content"].as_str().unwrap_or("").into())
    } else {
        if v["content_block"]["type"].as_str() == Some("tool_use") {
            return Err("Native tools are disabled; no requested tool was executed".into());
        }
        if v["type"].as_str() == Some("message_stop") {
            decoder.ended = true;
        }
        if v["type"].as_str() == Some("message_delta") {
            decoder.complete |= reason(v["delta"]["stop_reason"].as_str())?;
        }
        Ok(if v["delta"]["type"].as_str() == Some("text_delta") {
            v["delta"]["text"].as_str().unwrap_or("")
        } else {
            ""
        }
        .into())
    }
}
fn decode_full(protocol: Protocol, v: &Value) -> Result<String, String> {
    if v.get("error").is_some() {
        return Err("Native provider reported an error; response details omitted".into());
    }
    if protocol == Protocol::Chat {
        let choice = &v["choices"][0];
        if tools(&choice["message"]) {
            return Err("Native tools are disabled".into());
        }
        if choice["message"]
            .get("refusal")
            .is_some_and(|v| !v.is_null())
        {
            return Err("Native provider refused the request".into());
        }
        if !reason(choice["finish_reason"].as_str())? {
            return Err("Native response has no completion reason".into());
        }
        Ok(choice["message"]["content"]
            .as_str()
            .ok_or("Native response has no text")?
            .into())
    } else {
        if !reason(v["stop_reason"].as_str())? {
            return Err("Native response has no completion reason".into());
        }
        let mut text = String::new();
        for block in v["content"]
            .as_array()
            .ok_or("Native response has no content")?
        {
            if block["type"].as_str() != Some("text") {
                return Err("Unsupported native content; no tools executed".into());
            }
            text.push_str(block["text"].as_str().ok_or("Invalid native text block")?);
        }
        Ok(text)
    }
}
pub fn discover(config: Config) -> Result<Vec<String>, String> {
    config.validate()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "Could not start model discovery")?;
    let outcome = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(10), async {
            let response = request(&client()?, &config, "models", false)?
                .send()
                .await
                .map_err(|_| "Model discovery connection failed")?;
            if !response.status().is_success() {
                return Err(format!(
                    "Model discovery returned HTTP {}",
                    response.status().as_u16()
                ));
            }
            let mut bytes = Vec::new();
            let mut chunks = response.bytes_stream();
            while let Some(c) = chunks.next().await {
                bytes.extend_from_slice(&c.map_err(|_| "Model catalog was interrupted")?);
                if bytes.len() > 512 * 1024 {
                    return Err("Model catalog exceeds 512 KiB".into());
                }
            }
            let value: Value =
                serde_json::from_slice(&bytes).map_err(|_| "Invalid model catalog JSON")?;
            let mut models: Vec<String> = value["data"]
                .as_array()
                .ok_or("No model data array")?
                .iter()
                .take(500)
                .filter_map(|v| v["id"].as_str())
                .filter(|id| !id.is_empty() && id.len() <= 200 && !id.chars().any(char::is_control))
                .map(str::to_owned)
                .collect();
            models.sort();
            models.dedup();
            models.truncate(100);
            Ok(models)
        })
        .await
        .map_err(|_| "Model discovery timed out".to_owned())?
    });
    runtime.shutdown_background();
    outcome
}
