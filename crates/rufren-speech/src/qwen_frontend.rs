//! Conservative Qwen input policy. The contextual label is a text-shape
//! heuristic, never proof that a model chose the intended spoken stress.
use crate::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const ACUTE: char = '\u{301}';
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
fn russian(c: char) -> bool {
    matches!(c, 'А'..='я' | 'Ё' | 'ё')
}
fn vowel(c: char) -> bool {
    "аеёиоуыэюяАЕЁИОУЫЭЮЯ".contains(c)
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub(crate) fn input(text: &str) -> Result<SpeechSynthesisInput, SpeechError> {
    if !text.contains(ACUTE) {
        return Ok(SpeechSynthesisInput {
            source_text_sha256: hash(text),
            synthesis_text: text.into(),
            synthesis_text_sha256: hash(text),
            transform: SpeechTextTransform::Verbatim,
            context: SpeechSynthesisContext::OrdinaryText,
        });
    }
    let words: Vec<_> = text
        .split(|c| !russian(c) && c != ACUTE)
        .filter(|s| !s.is_empty())
        .collect();
    // This deliberately excludes short greetings and minimal stress drills.
    // A larger sentence does not establish correct model pronunciation.
    if words.len() < 8
        || !text
            .trim_end()
            .chars()
            .last()
            .is_some_and(|c| matches!(c, '.' | '?' | '!' | '…'))
    {
        return Err(SpeechError::UnsupportedStress);
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for word in words {
        if !word.contains(ACUTE) {
            continue;
        }
        let mut base = String::new();
        let mut position = None;
        let mut previous = None;
        let mut letters = 0usize;
        for c in word.chars() {
            if c == ACUTE {
                if position.is_some() || !previous.is_some_and(vowel) {
                    return Err(SpeechError::UnsupportedStress);
                }
                position = Some(letters.saturating_sub(1));
            } else {
                base.extend(c.to_lowercase());
                letters += 1;
            }
            previous = Some(c);
        }
        let position = position.ok_or(SpeechError::UnsupportedStress)?;
        if AMBIGUOUS.contains(&base.as_str()) || seen.get(&base).is_some_and(|p| *p != position) {
            return Err(SpeechError::UnsupportedStress);
        }
        seen.insert(base, position);
    }
    let synthesis_text: String = text.chars().filter(|c| *c != ACUTE).collect();
    Ok(SpeechSynthesisInput {
        source_text_sha256: hash(text),
        synthesis_text_sha256: hash(&synthesis_text),
        synthesis_text,
        transform: SpeechTextTransform::RemoveCombiningAcuteInContext,
        context: SpeechSynthesisContext::ContextualUtterance,
    })
}

/// Convert authored acute marks to Silero's explicit stress syntax. This does
/// not ask Silero to guess a homograph and does not certify its acoustic output.
pub(crate) fn silero_input(text: &str) -> Result<SpeechSynthesisInput, SpeechError> {
    if !text.contains(ACUTE) || text.contains('+') {
        return Err(SpeechError::UnsupportedStress);
    }
    let mut synthesis_text = String::new();
    let mut marked_in_word = false;
    for c in text.chars() {
        if c == ACUTE {
            let previous = synthesis_text.pop().ok_or(SpeechError::UnsupportedStress)?;
            if !vowel(previous) || marked_in_word {
                return Err(SpeechError::UnsupportedStress);
            }
            synthesis_text.push('+');
            synthesis_text.push(previous);
            marked_in_word = true;
        } else {
            if !russian(c) {
                marked_in_word = false;
            }
            synthesis_text.push(c);
        }
    }
    Ok(SpeechSynthesisInput {
        source_text_sha256: hash(text),
        synthesis_text_sha256: hash(&synthesis_text),
        synthesis_text,
        transform: SpeechTextTransform::PedagogicalAcuteToSileroPlus,
        context: SpeechSynthesisContext::ExplicitStress,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_synthesis_provenance_checks_exact_transforms_hashes_and_context() {
        for (source, expected) in [
            ("Это за́мок.", "Это з+амок."),
            ("Это замо́к.", "Это зам+ок."),
            ("Ёлка и ё́ж.", "Ёлка и +ёж."),
        ] {
            let input = silero_input(source).unwrap();
            assert_eq!(input.synthesis_text, expected);
            input.validate_for(source).unwrap();
            let mut false_input = input.clone();
            false_input.synthesis_text = expected.replace('ё', "е");
            if false_input.synthesis_text != input.synthesis_text {
                assert!(false_input.validate_for(source).is_err());
            }
            false_input = input.clone();
            false_input.context = SpeechSynthesisContext::ContextualUtterance;
            assert!(false_input.validate_for(source).is_err());
            false_input = input.clone();
            false_input.source_text_sha256 = "0".repeat(64);
            assert!(false_input.validate_for(source).is_err());
        }
        assert!(silero_input("Это за́мо́к.").is_err());
        assert!(silero_input("Это з́амок.").is_err());
        let source = "Это за́мок.";
        let forged = SpeechSynthesisInput {
            source_text_sha256: hash(source),
            synthesis_text: "Это замок.".into(),
            synthesis_text_sha256: hash("Это замок."),
            transform: SpeechTextTransform::RemoveCombiningAcuteInContext,
            context: SpeechSynthesisContext::ContextualUtterance,
        };
        assert!(forged.validate_for(source).is_err());
    }

    #[test]
    fn speech_qwen_frontend_preserves_exact_source_and_rejects_isolated_stress() {
        for text in [
            "Это за́мок.",
            "Это замо́к.",
            "Приве́т!",
            "Это долгая фраза про старый за́мок, который стоит на холме.",
        ] {
            assert!(
                matches!(input(text), Err(SpeechError::UnsupportedStress)),
                "{text}"
            );
        }
        let plain = "Всё готово? Для каждого натурального числа существует следующее число.";
        let result = input(plain).unwrap();
        assert_eq!(result.synthesis_text, plain);
        assert_eq!(result.transform, SpeechTextTransform::Verbatim);
        let marked = "Сего́дня я изуча́ю ру́сский язы́к и чита́ю интере́сную кни́гу о звёздах.";
        let result = input(marked).unwrap();
        assert_eq!(result.source_text_sha256, hash(marked));
        assert_eq!(
            result.synthesis_text,
            "Сегодня я изучаю русский язык и читаю интересную книгу о звёздах."
        );
        assert_eq!(result.synthesis_text_sha256, hash(&result.synthesis_text));
        assert_eq!(result.context, SpeechSynthesisContext::ContextualUtterance);
    }
}
