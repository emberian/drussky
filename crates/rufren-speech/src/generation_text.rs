//! New-generation admission only. Historical frontend proofs remain unchanged.
#[cfg(any(feature = "server", test))]
use crate::SpeechError;

#[cfg(any(feature = "server", test))]
const ACUTE: char = '\u{301}';
const KNOWN_AMBIGUOUS: &str = include_str!("../assets/known_ambiguous_forms.txt");

pub(crate) fn known_ambiguous(word: &str) -> bool {
    KNOWN_AMBIGUOUS.lines().any(|known| known == word)
}

#[cfg(any(feature = "server", test))]
fn russian(c: char) -> bool {
    matches!(c, 'А'..='я' | 'Ё' | 'ё')
}

#[cfg(any(feature = "server", test))]
pub(crate) fn validate(text: &str) -> Result<(), SpeechError> {
    if text.chars().any(russian) && text.chars().any(char::is_numeric) {
        return Err(SpeechError::SpokenFormRequired);
    }
    for token in text
        .split(|c| !russian(c) && c != ACUTE)
        .filter(|s| !s.is_empty())
    {
        let base: String = token
            .chars()
            .filter(|c| *c != ACUTE)
            .flat_map(char::to_lowercase)
            .collect();
        if !known_ambiguous(&base) {
            continue;
        }
        let mut previous = None;
        let mut marks = 0;
        for c in token.chars() {
            if c == ACUTE {
                marks += 1;
                if marks > 1 || !previous.is_some_and(|v| "аеёиоуыэюяАЕЁИОУЫЭЮЯ".contains(v))
                {
                    return Err(SpeechError::UnsupportedStress);
                }
            }
            previous = Some(c);
        }
        if marks == 0 {
            return Err(SpeechError::AmbiguousStress);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_generation_guard_checks_every_known_token_at_any_length() {
        assert_eq!(KNOWN_AMBIGUOUS.lines().count(), 15);
        for word in KNOWN_AMBIGUOUS.lines() {
            for text in [
                word.to_owned(),
                format!("Сего́дня мы вместе читаем длинное предложение про {word}."),
                format!("{word}, {word}!"),
            ] {
                assert!(
                    matches!(validate(&text), Err(SpeechError::AmbiguousStress)),
                    "{text}"
                );
            }
        }
        for text in [
            "Это за́мок, а это замок.",
            "За́мок — ЗАМОК",
            "Приве́т! Это замок.",
        ] {
            assert!(matches!(validate(text), Err(SpeechError::AmbiguousStress)));
        }
        for text in ["Это з́амок.", "Это за́мо́к.", "́замок"] {
            assert!(matches!(
                validate(text),
                Err(SpeechError::UnsupportedStress)
            ));
        }
        for text in [
            "Это за́мок, а это замо́к.",
            "ЗА́МОК!",
            "О замках.",
            "Здравствуйте!",
        ] {
            assert!(validate(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn speech_generation_guard_requires_authored_numeric_readout_without_rewriting() {
        for text in [
            "У меня 21 свободная минута.",
            "У меня ２ вопроса.",
            "У меня ² вопроса.",
            "У меня ½ минуты.",
            "У меня́ 2 вопроса.",
        ] {
            assert!(matches!(
                validate(text),
                Err(SpeechError::SpokenFormRequired)
            ));
        }
        assert!(validate("У меня двадцать одна свободная минута.").is_ok());
        assert!(validate("Chapter 21").is_ok());
        // These are still valid historical provenance proofs, not permission to regenerate.
        for old in [
            "На двери висит замок, а на холме стоит старый замок.",
            "У меня 21 свободная минута.",
        ] {
            crate::qwen_frontend::input(old)
                .unwrap()
                .validate_for(old)
                .unwrap();
        }
    }
}
