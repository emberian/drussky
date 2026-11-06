//! Explicit local routing, selected before inference; never a failure fallback.
use crate::{
    SpeechError, SpeechRequest, SpeechResponse, qwen_local::QwenLocal, silero_local::SileroLocal,
};

pub(crate) const STRATEGY: &str = "rufren-local-russian-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    ExplicitStress,
    RussianFrontend,
    ContextualQwen,
}

pub(crate) fn route(text: &str) -> Result<Route, SpeechError> {
    // A literal plus is Silero control syntax, not supported authored speech.
    if text.contains('+') {
        return Err(SpeechError::InvalidRequest);
    }
    if text.contains('\u{301}') {
        crate::qwen_frontend::silero_input(text)?.validate_for(text)?;
        return Ok(Route::ExplicitStress);
    }
    let words: Vec<String> = text
        .split(|c| !matches!(c, 'А'..='я' | 'Ё' | 'ё'))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    const AMBIGUOUS: &[&str] = &[
        "замок",
        "мука",
        "плачу",
        "атлас",
        "орган",
        "духи",
        "ирис",
        "белки",
        "стрелки",
        "кружки",
        "стоит",
        "дороги",
        "вести",
        "пропасть",
        "село",
    ];
    // Word count and punctuation classify input shape, not semantic resolution.
    let contextual = words.len() >= 8 && text.trim_end().ends_with(['.', '?', '!', '…']);
    if !contextual && words.iter().any(|word| AMBIGUOUS.contains(&word.as_str())) {
        return Err(SpeechError::AmbiguousStress);
    }
    if !contextual {
        Ok(Route::RussianFrontend)
    } else {
        Ok(Route::ContextualQwen)
    }
}

#[derive(Clone)]
pub(crate) struct LocalRussian {
    qwen: QwenLocal,
    silero: SileroLocal,
}
impl LocalRussian {
    pub(crate) fn installed(qwen: QwenLocal) -> Result<Self, SpeechError> {
        Ok(Self {
            qwen,
            silero: SileroLocal::installed()?,
        })
    }
    pub(crate) async fn ready(&self) -> bool {
        self.silero.ready()
            && self
                .qwen
                .status()
                .await
                .is_ok_and(|s| s.loaded && s.enabled)
    }
    pub(crate) async fn generate(
        &self,
        request: &SpeechRequest,
    ) -> Result<SpeechResponse, SpeechError> {
        let selected = route(&request.text)?;
        let mut response = match selected {
            Route::ExplicitStress => self.silero.generate(request, true).await?,
            Route::RussianFrontend => self.silero.generate(request, false).await?,
            Route::ContextualQwen => self.qwen.generate(request).await?,
        };
        response.provenance.validation.push_str(match selected {
            Route::ExplicitStress => "; router rufren-local-russian-v1 selected authored explicit stress; audible stress remains unverified",
            Route::RussianFrontend => "; router rufren-local-russian-v1 selected short ordinary Russian input; automatic stress is a model choice, not certified disambiguation",
            Route::ContextualQwen => "; router rufren-local-russian-v1 selected contextual ordinary input by text shape, not a pronunciation judgment",
        });
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_hybrid_routes_before_inference_without_erasing_stress() {
        for text in ["Это за́мок.", "Это замо́к.", "Приве́т!"] {
            assert_eq!(route(text).unwrap(), Route::ExplicitStress);
        }
        for text in [
            "Здравствуйте!",
            "Мы сегодня вместе читаем интересную русскую книгу о звёздах",
        ] {
            assert_eq!(route(text).unwrap(), Route::RussianFrontend);
        }
        assert_eq!(
            route("Всё готово? Для каждого натурального числа существует следующее число.")
                .unwrap(),
            Route::ContextualQwen
        );
        assert_eq!(
            route("На двери висит замок, а на холме стоит старый замок.").unwrap(),
            Route::ContextualQwen
        );
        assert!(matches!(
            route("Это замок."),
            Err(SpeechError::AmbiguousStress)
        ));
        assert!(route("Это з+амок.").is_err());
        assert!(route("Это з́амок.").is_err());
    }
}
