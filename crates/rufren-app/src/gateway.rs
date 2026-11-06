//! Bounded private-gateway transport shared by teaching and generated speech.
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::document;
use serde::de::DeserializeOwned;

const ERROR_BODY_LIMIT: usize = 2048;

fn relay_base(endpoint: &str, token: &str) -> Result<String, String> {
    let base = endpoint.trim().trim_end_matches('/').to_owned();
    let url = reqwest::Url::parse(&base)
        .map_err(|_| "Add your private relay URL in Settings first.".to_string())?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("Use an HTTPS private relay, or localhost for development.".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use a relay URL without credentials, query, or fragment.".into());
    }
    if token.trim().is_empty() {
        return Err("Add your private connection key in Settings first.".into());
    }
    Ok(base)
}

fn http_problem(status: u16) -> String {
    match status {
        401|403=>"Your private relay did not accept the connection key. Check it in Settings.",
        400|422=>"The relay could not accept this request. Check the selected text or recording.",
        413=>"This passage or recording exceeds the relay’s size limit. Choose a shorter one.",
        415=>"This recording format is not supported. Use a PCM16 or float32 WAV, mono or stereo, at 8–48 kHz.",
        429=>"The relay or provider is busy, or this relay session’s AI budget is used up. Check availability before trying again.",
        408|504=>"That took too long. Your work is still here; you can try again.",
        503=>"The teacher needs relay setup, a valid provider credential, or available account credit. Check availability and the relay configuration.",
        _=>"The service is unavailable right now. Your work and the previous response are still here.",
    }.into()
}

// Never display arbitrary relay/provider bodies. These exact public messages
// carry actionable speech guidance without exposing upstream diagnostics.
fn public_problem(status: u16, body: &[u8]) -> String {
    if body.len() <= ERROR_BODY_LIMIT {
        use rufren_speech::{RecognitionError, SpeechError};
        let allowed = match status {
            400 => vec![
                SpeechError::UnsupportedStress.to_string(),
                SpeechError::AmbiguousStress.to_string(),
                RecognitionError::HashMismatch.to_string(),
            ],
            409 => vec![RecognitionError::Cancelled.to_string()],
            415 => vec![RecognitionError::UnsupportedAudio.to_string()],
            _ => Vec::new(),
        };
        if let Some(message) = allowed
            .into_iter()
            .find(|message| message.as_bytes() == body)
        {
            return message;
        }
    }
    http_problem(status)
}

pub(crate) async fn request<T: DeserializeOwned>(
    endpoint: &str,
    token: &str,
    path: &str,
    body: Option<&serde_json::Value>,
    response_limit: usize,
    timeout_ms: u64,
) -> Result<T, String> {
    let base = relay_base(endpoint, token)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let client =
            crate::native_http::client(&base, std::time::Duration::from_millis(timeout_ms))?;
        let request = if let Some(body) = body {
            client.post(format!("{base}{path}")).json(body)
        } else {
            client.get(format!("{base}{path}"))
        };
        let mut response = request.bearer_auth(token).send().await.map_err(|e| {
            if e.is_timeout() {
                http_problem(504)
            } else {
                "Could not reach your private relay. Check its URL and connection.".into()
            }
        })?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let mut detail = Vec::new();
            if response
                .content_length()
                .is_none_or(|n| n <= ERROR_BODY_LIMIT as u64)
            {
                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) if detail.len() + chunk.len() <= ERROR_BODY_LIMIT => {
                            detail.extend_from_slice(&chunk)
                        }
                        Ok(None) => break,
                        _ => {
                            detail.clear();
                            break;
                        }
                    }
                }
            }
            return Err(public_problem(status, &detail));
        }
        if response
            .content_length()
            .is_some_and(|n| n > response_limit as u64)
        {
            return Err("The relay response exceeded the size limit.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Could not finish reading the relay response.")?
        {
            if bytes.len() + chunk.len() > response_limit {
                return Err("The relay response exceeded the size limit.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "The relay returned an unreadable response.".into())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({"url":format!("{base}{path}"),"token":token,"body":body.map(serde_json::to_string).transpose().map_err(|_|"Could not prepare this text.")?,"limit":response_limit,"error_limit":ERROR_BODY_LIMIT,"timeout":timeout_ms});
        let script = format!("const a={args};\n{}", include_str!("gateway_fetch.js"));
        let result: serde_json::Value = document::eval(&script)
            .await
            .map_err(|_| "Could not initialize the relay connection.")?;
        if let Some(status) = result.get("status").and_then(|s| s.as_u64()) {
            return Err(public_problem(
                status as u16,
                result
                    .get("detail")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .as_bytes(),
            ));
        }
        if let Some(problem) = result.get("error").and_then(|s| s.as_str()) {
            return Err(match problem {"timeout"=>http_problem(504),"large"=>"The relay response exceeded the size limit.".into(),_=>"Could not reach your private relay. Check its URL, connection and allowed origins.".into()});
        }
        serde_json::from_str(
            result
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("The relay returned an empty response.")?,
        )
        .map_err(|_| "The relay returned an unreadable response.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rufren_speech::{RecognitionError, SpeechError};

    #[test]
    fn gateway_public_speech_errors_preserve_actionable_guidance() {
        for (status, message) in [
            (400, SpeechError::UnsupportedStress.to_string()),
            (400, SpeechError::AmbiguousStress.to_string()),
            (400, RecognitionError::HashMismatch.to_string()),
            (409, RecognitionError::Cancelled.to_string()),
            (415, RecognitionError::UnsupportedAudio.to_string()),
        ] {
            assert_eq!(public_problem(status, message.as_bytes()), message);
        }
    }

    #[test]
    fn gateway_untrusted_bodies_never_reach_the_user() {
        let known = SpeechError::AmbiguousStress.to_string();
        for body in [
            b"provider credential=private-fixture".to_vec(),
            format!("{known}\nprivate fixture").into_bytes(),
            format!(" {known}").into_bytes(),
            vec![b'x'; ERROR_BODY_LIMIT + 1],
            vec![0xff],
        ] {
            assert_eq!(public_problem(400, &body), http_problem(400));
        }
        assert_eq!(public_problem(401, known.as_bytes()), http_problem(401));
        assert!(!http_problem(400).contains("shorter"));
    }
}
