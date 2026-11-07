//! Sparse sound-to-writing returns inside the prepared course.
//!
//! This additive plan leaves the original teaching steps and writing definitions
//! unchanged. A position in the plan is not evidence that prerequisites were met:
//! the host must require the writing task's actual prerequisite encounters.
//! Instructions are safe before an answer. Comparison text is answer-bearing and
//! belongs only after the learner saves an attempt or explicitly asks for help.

pub const HANDWRITING_PRACTICE_PLAN_REVISION: &str = "foundation-writing-practice-1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointSpec {
    pub id: &'static str,
    pub revision: &'static str,
    /// The checkpoint follows this completed step; it does not replace its task.
    pub after_step_id: &'static str,
    pub task_id: &'static str,
    pub instruction: &'static str,
    /// A comparison prompt, never an automatic judgment of the learner's ink.
    pub comparison_focus: &'static str,
}

const SYLLABLE: &str = "Listen, then write the whole sound pattern in lowercase. You can replay or ask for the example.";
const WORD: &str =
    "Listen, then write the word in lowercase. You can replay or ask for the example.";
const SENTENCE: &str = "Listen, then write the sentence. Start with a capital, leave spaces between words and finish with a period.";
const QUESTION: &str = "Listen, then write the question. Start with a capital, leave spaces between words and finish with a question mark.";

// Selected returns follow intervening material. They are not a required second
// round after every copied item. The audio-only fourth episode has no writing
// checkpoint because its new spellings have not yet been introduced.
const PLAN: &[CheckpointSpec] = &[
    CheckpointSpec {
        id: "write-recall-ma",
        revision: "1",
        after_step_id: "f1-radio-no",
        task_id: "foundation-hand-print-ma",
        instruction: SYLLABLE,
        comparison_focus: "Follow м → а from left to right. Compare the order of your two letters with the example.",
    },
    CheckpointSpec {
        id: "write-recall-am",
        revision: "1",
        after_step_id: "f1-radio-no",
        task_id: "foundation-hand-print-am",
        instruction: SYLLABLE,
        comparison_focus: "This pattern starts with а and ends with м. Compare that order with your drawing, then listen again.",
    },
    CheckpointSpec {
        id: "write-recall-kot",
        revision: "1",
        after_step_id: "f2-radio-no",
        task_id: "foundation-hand-print-kot",
        instruction: WORD,
        comparison_focus: "Compare к–о–т in order: beginning, middle vowel, final letter. All three belong to one word.",
    },
    CheckpointSpec {
        id: "write-recall-tam-kot",
        revision: "1",
        after_step_id: "f3-meaning-there",
        task_id: "foundation-hand-print-tam-kot",
        instruction: SENTENCE,
        comparison_focus: "Там tells where; кот tells what is there. Check the capital Т, the space between the two words and the final period.",
    },
    CheckpointSpec {
        id: "write-recall-c4-he-script",
        revision: "1",
        after_step_id: "c4-he-listen",
        task_id: "foundation-hand-print-on",
        instruction: WORD,
        comparison_focus: "Compare о followed by н. This is the word for the man in our scene; it has no final а.",
    },
    CheckpointSpec {
        id: "write-recall-c5-window-read",
        revision: "1",
        after_step_id: "c5-window-read-return",
        task_id: "foundation-hand-print-okno",
        instruction: WORD,
        comparison_focus: "Compare the four letters о–к–н–о. Keep the middle к and н together without adding a vowel.",
    },
    CheckpointSpec {
        id: "write-recall-c6-ni-context",
        revision: "1",
        after_step_id: "c6-window-it-say",
        task_id: "foundation-hand-print-oni",
        instruction: WORD,
        comparison_focus: "Check the final и in они. This word refers to the people together, rather than one man or one woman.",
    },
    CheckpointSpec {
        id: "write-recall-a8-you-there",
        revision: "1",
        after_step_id: "a8-you-listen",
        task_id: "foundation-hand-print-a8-you-there",
        instruction: SENTENCE,
        comparison_focus: "Ты names the familiar listener; там gives the location. Compare the two word groups, capital and period.",
    },
    CheckpointSpec {
        id: "write-recall-a9-this-cat",
        revision: "1",
        after_step_id: "a9-this-window-listen",
        task_id: "foundation-hand-print-a9-this-cat",
        instruction: SENTENCE,
        comparison_focus: "Это starts the naming message; кот names the cat. Compare the letters in each word and the space between them.",
    },
    CheckpointSpec {
        id: "write-recall-a10-table",
        revision: "1",
        after_step_id: "a10-juice-listen",
        task_id: "foundation-hand-print-a10-table",
        instruction: WORD,
        comparison_focus: "Compare с–т–о–л. The first two consonants stay together: there is no written vowel between с and т.",
    },
    CheckpointSpec {
        id: "write-recall-a11-hand",
        revision: "1",
        after_step_id: "a11-location-listen",
        task_id: "foundation-hand-print-a11-hand",
        instruction: WORD,
        comparison_focus: "Compare р–у–к–а in order. Check the beginning and the final vowel while replaying the whole word.",
    },
    CheckpointSpec {
        id: "write-recall-a12-water",
        revision: "1",
        after_step_id: "a12-we-listen",
        task_id: "foundation-hand-print-a12-water",
        instruction: WORD,
        comparison_focus: "The spelling is вода: written о in the first syllable, а in the last. The last vowel is stressed; compare the whole sound with the spelling.",
    },
    CheckpointSpec {
        id: "write-recall-a13-robot",
        revision: "1",
        after_step_id: "a13-dad-listen",
        task_id: "foundation-hand-print-a13-robot",
        instruction: WORD,
        comparison_focus: "робот has two written о letters. The first vowel is stressed; compare the middle б and final т too.",
    },
    CheckpointSpec {
        id: "write-recall-a14-book",
        revision: "1",
        after_step_id: "a14-book-reference",
        task_id: "foundation-hand-print-a14-book",
        instruction: WORD,
        comparison_focus: "Compare к–н–и–г–а. Keep the first two consonants together and check the final а.",
    },
    CheckpointSpec {
        id: "write-recall-a15-ball",
        revision: "1",
        after_step_id: "a15-beetle-listen",
        task_id: "foundation-hand-print-a15-ball",
        instruction: WORD,
        comparison_focus: "Compare ш–а–р. Notice the three uprights in the printed ш and the final р.",
    },
    CheckpointSpec {
        id: "write-recall-a16-hatch",
        revision: "1",
        after_step_id: "a16-honey-listen",
        task_id: "foundation-hand-print-a16-hatch",
        instruction: WORD,
        comparison_focus: "люк uses ю, while лук uses у. Compare your middle letter and replay the soft beginning of the hatch word.",
    },
    CheckpointSpec {
        id: "write-recall-a17-my-cat",
        revision: "1",
        after_step_id: "a17-salt-write",
        task_id: "foundation-hand-print-a17-my-cat",
        instruction: "Listen, then write the two-word phrase in lowercase with a space. No final punctuation is needed for this phrase.",
        comparison_focus: "мой ends in й, including its short mark above. Compare both words and their space; this is the form used with кот.",
    },
    CheckpointSpec {
        id: "write-recall-a18-tea",
        revision: "1",
        after_step_id: "a18-read-circus",
        task_id: "foundation-hand-print-a18-tea",
        instruction: WORD,
        comparison_focus: "Compare ч–а–й. Keep the short mark above й; the last letter is a glide, not another vowel syllable.",
    },
    CheckpointSpec {
        id: "write-recall-a19-what-this",
        revision: "1",
        after_step_id: "a19-object-write",
        task_id: "foundation-hand-print-a19-what-this",
        instruction: QUESTION,
        comparison_focus: "The question is Что это? Compare the spelling of Что, the separate word это and the question mark.",
    },
    CheckpointSpec {
        id: "write-recall-a20-we-read",
        revision: "1",
        after_step_id: "a20-i-think",
        task_id: "foundation-hand-print-a20-we-read",
        instruction: SENTENCE,
        comparison_focus: "Мы includes the speaker in a group. Compare the ending -ем in читаем, then the space and final period.",
    },
    CheckpointSpec {
        id: "write-recall-a20-you-think",
        revision: "1",
        after_step_id: "a20-i-read-say",
        task_id: "foundation-hand-print-a20-you-think",
        instruction: SENTENCE,
        comparison_focus: "Ты addresses one familiar listener. Compare the ending -ешь in думаешь; keep the final ь in writing.",
    },
    CheckpointSpec {
        id: "write-recall-a21-fact-question",
        revision: "1",
        after_step_id: "a21-read-math",
        task_id: "foundation-hand-print-a21-fact-question",
        instruction: QUESTION,
        comparison_focus: "Это факт? asks whether this is a fact. Compare ф–а–к–т, the word space and the final question mark.",
    },
];

pub fn handwriting_practice_plan() -> &'static [CheckpointSpec] {
    PLAN
}

pub fn checkpoints_after(step_id: &str) -> Vec<&'static CheckpointSpec> {
    PLAN.iter()
        .filter(|spec| spec.after_step_id == step_id)
        .collect()
}

pub fn checkpoint(id: &str) -> Option<&'static CheckpointSpec> {
    PLAN.iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handwriting_tasks::{
        FoundationHandwritingCase, FoundationHandwritingInputFocus, FoundationHandwritingSequence,
        handwriting_tasks,
    };
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn checkpoints_resolve_after_intervening_material_with_taught_prerequisites() {
        let pack = crate::foundations();
        let tasks = handwriting_tasks();
        let steps: Vec<_> = pack.episodes.iter().flat_map(|e| &e.steps).collect();
        let mut ids = BTreeSet::new();
        let mut counts = BTreeMap::new();
        for spec in PLAN {
            assert!(ids.insert(spec.id), "duplicate {}", spec.id);
            assert_eq!(spec.revision, "1");
            let anchor = steps
                .iter()
                .position(|s| s.id == spec.after_step_id)
                .unwrap();
            let task = tasks.iter().find(|t| t.id == spec.task_id).unwrap();
            let source = steps
                .iter()
                .position(|s| s.id == task.foundation_step_id)
                .unwrap();
            assert!(
                anchor > source + 1,
                "{} needs intervening material",
                spec.id
            );
            assert_eq!(
                task.input_focus,
                FoundationHandwritingInputFocus::WholeTarget
            );
            assert!(
                task.sequences
                    .contains(&FoundationHandwritingSequence::HearWriteCompareSay)
            );
            for prerequisite in &task.prerequisite_step_ids {
                let position = steps.iter().position(|s| &s.id == prerequisite).unwrap();
                assert!(position < anchor, "{} has a future prerequisite", spec.id);
            }
            let episode = pack
                .episodes
                .iter()
                .find(|e| e.steps.iter().any(|s| s.id == spec.after_step_id))
                .unwrap();
            assert!(
                episode
                    .steps
                    .iter()
                    .any(|s| s.id == task.foundation_step_id)
            );
            *counts.entry(episode.id.as_str()).or_insert(0) += 1;
        }
        assert_eq!(counts.len(), 20);
        assert!(
            !counts.contains_key("foundation-4"),
            "audio-only words lack taught spelling"
        );
        assert!(counts.values().all(|n| *n <= 2), "keep additions sparse");
    }

    #[test]
    fn hidden_prompts_do_not_disclose_spelling_and_state_unhearable_conventions() {
        let tasks = handwriting_tasks();
        for spec in PLAN {
            assert!(
                !spec
                    .instruction
                    .chars()
                    .any(|c| ('А'..='я').contains(&c) || c == 'Ё' || c == 'ё'),
                "{} leaks script",
                spec.id
            );
            assert!(!spec.comparison_focus.trim().is_empty());
            let task = tasks.iter().find(|t| t.id == spec.task_id).unwrap();
            match task.case {
                FoundationHandwritingCase::Uppercase => {
                    assert!(spec.instruction.contains("capital"))
                }
                FoundationHandwritingCase::Lowercase => {
                    assert!(spec.instruction.contains("lowercase"))
                }
                FoundationHandwritingCase::Sentence => {
                    // Legacy phrase definitions also use Sentence for lowercase
                    // fragments such as мой кот. Preserve that definition and
                    // instruct the actual taught form, not an invented capital.
                    if task.target.chars().next().unwrap().is_uppercase() {
                        assert!(spec.instruction.contains("capital"));
                    } else {
                        assert!(spec.instruction.contains("lowercase"));
                    }
                }
            }
            if task.target.ends_with('.') {
                assert!(
                    spec.instruction.contains("period"),
                    "{} leaves punctuation implicit",
                    spec.id
                );
            }
            if task.target.ends_with('?') {
                assert!(spec.instruction.contains("question mark"));
            }
            if task.target.contains(' ') {
                assert!(spec.instruction.contains("space"));
            }
        }
    }

    #[test]
    fn opening_progresses_from_sound_patterns_to_a_word_and_message() {
        let first = checkpoints_after("f1-radio-no");
        assert_eq!(
            first.iter().map(|s| s.task_id).collect::<Vec<_>>(),
            vec!["foundation-hand-print-ma", "foundation-hand-print-am"]
        );
        assert_eq!(
            checkpoints_after("f2-radio-no")[0].task_id,
            "foundation-hand-print-kot"
        );
        assert_eq!(
            checkpoints_after("f3-meaning-there")[0].task_id,
            "foundation-hand-print-tam-kot"
        );
        for spec in PLAN {
            assert_eq!(checkpoint(spec.id), Some(spec));
        }
        assert!(checkpoint("unknown-checkpoint").is_none());
        assert!(
            checkpoints_after("f1-a").is_empty(),
            "do not force immediate per-copy dictation"
        );
    }
}
