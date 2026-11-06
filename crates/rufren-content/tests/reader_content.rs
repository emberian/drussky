use rufren_content::{ContentError, ReaderAnthology, load_embedded, load_reader_anthology};
use rufren_core::{Fact, Library, ReaderSourceKind};
use std::collections::BTreeSet;

#[test]
fn reader_content_inventory_preserves_originals_sources_and_separate_ai_help() {
    let anthology = load_reader_anthology().unwrap();
    assert_eq!(anthology.schema_version, 1);
    assert_eq!(anthology.id, "ru.reader.anthology");
    assert_eq!(anthology.version, "1");
    assert_eq!(anthology.documents.len(), 4);
    let expected = [
        ("ru.reader.echo", 10),
        ("ru.reader.no_largest", 8),
        ("ru.reader.words_and_evidence", 8),
        ("ru.reader.chekhov_student_opening", 5),
    ];
    for (id, chunk_count) in expected {
        let definition = anthology.definition(id, "1").unwrap();
        definition.validate().unwrap();
        assert_eq!(definition.chunks.len(), chunk_count);
        assert_eq!(definition.original_sha256.len(), 64);
        assert!(!definition.source.title.trim().is_empty());
        assert!(!definition.source.author.trim().is_empty());
        assert!(!definition.source.rights.trim().is_empty());
        assert!(!definition.source.provenance.trim().is_empty());
        assert!(definition.help_provenance.to_lowercase().contains("ai"));
        assert!(definition.chunks.iter().any(|chunk| chunk.stress.is_some()));
        assert!(
            definition
                .chunks
                .iter()
                .any(|chunk| chunk.translation.is_some())
        );
        assert!(
            definition
                .chunks
                .iter()
                .map(|chunk| chunk.prompts.len())
                .sum::<usize>()
                >= 2
        );
        let mut cursor = 0;
        let mut reconstructed = String::new();
        for chunk in &definition.chunks {
            assert_eq!(chunk.span.start, cursor);
            assert!(definition.original.is_char_boundary(chunk.span.start));
            assert!(definition.original.is_char_boundary(chunk.span.end));
            assert_eq!(
                &definition.original[chunk.span.start..chunk.span.end],
                chunk.text
            );
            reconstructed.push_str(&chunk.text);
            cursor = chunk.span.end;
            if let Some(stress) = &chunk.stress {
                assert_eq!(
                    stress
                        .chars()
                        .filter(|c| *c != '\u{301}')
                        .collect::<String>(),
                    chunk.text,
                    "the support layer must preserve exact spelling, punctuation and whitespace"
                );
            }
            for note in chunk.glosses.iter().chain(&chunk.grammar_notes) {
                assert!(note.span.start >= chunk.span.start && note.span.end <= chunk.span.end);
                assert_eq!(
                    &definition.original[note.span.start..note.span.end],
                    note.text
                );
            }
        }
        assert_eq!(cursor, definition.original.len());
        assert_eq!(reconstructed, definition.original);
        if id == "ru.reader.chekhov_student_opening" {
            assert_eq!(definition.source.kind, ReaderSourceKind::AuthenticExcerpt);
            assert!(
                definition
                    .source
                    .url
                    .as_deref()
                    .is_some_and(|url| url.starts_with("https://feb-web.ru/feb/chekhov/texts/"))
            );
            assert_eq!(
                definition.original,
                "Погода вначале была хорошая, тихая. Кричали дрозды, и по соседству в болотах что-то живое жалобно гудело, точно дуло в пустую бутылку."
            );
        } else {
            assert_eq!(definition.source.kind, ReaderSourceKind::AiAuthored);
        }
        assert!(anthology.definition(id, "unknown-version").is_none());
        assert!(anthology.document(id, "unknown-version").is_err());
    }
    assert_eq!(load_embedded().unwrap().revision, "2026-11-05.4");
}

#[test]
fn reader_content_builders_emit_deterministic_full_documents_and_events() {
    let anthology = load_reader_anthology().unwrap();
    let first = anthology.document_events().unwrap();
    let second = anthology.document_events().unwrap();
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    let mut ids = BTreeSet::new();
    let mut library = Library::default();
    library.merge(&first).unwrap();
    library.merge(&second).unwrap();
    assert_eq!(library.events.len(), 4);
    for (definition, event) in anthology.documents.iter().zip(first) {
        assert_eq!(
            event.version, 7,
            "published deterministic Reader envelopes retain their original version"
        );
        assert_eq!(event.recorded_at_ms, 0);
        let document = anthology
            .document(&definition.id, &definition.version)
            .unwrap();
        assert!(ids.insert(document.document_id));
        assert_eq!(document.definition, *definition);
        assert_eq!(
            anthology
                .document_event(&definition.id, &definition.version)
                .unwrap(),
            event
        );
        let Fact::ReaderDocumentRecorded(recorded) = event.fact else {
            panic!("Reader document event expected")
        };
        assert_eq!(recorded, document);
    }
}

#[test]
fn reader_content_rejects_missing_coverage_changed_original_and_false_stress_layer() {
    let original = load_reader_anthology().unwrap();
    for mode in 0..5 {
        let mut bad = original.clone();
        match mode {
            0 => {
                bad.documents[0].chunks.pop();
            }
            1 => bad.documents[0].chunks[0].text.push(' '),
            2 => bad.documents[0].original.push(' '),
            3 => {
                bad.documents[0].chunks[0].stress =
                    Some(format!("{}!", bad.documents[0].chunks[0].text))
            }
            _ => {
                let chunk = bad
                    .documents
                    .iter_mut()
                    .flat_map(|definition| &mut definition.chunks)
                    .find(|chunk| chunk.text.contains('е'))
                    .unwrap();
                chunk.stress = Some(chunk.text.replacen('е', "ё", 1));
            }
        }
        assert!(
            bad.validate().is_err(),
            "bad original/chunk/stress case {mode}"
        );
    }
    let mut bad = original;
    let chunk = bad
        .documents
        .iter_mut()
        .flat_map(|definition| &mut definition.chunks)
        .find(|chunk| !chunk.glosses.is_empty())
        .unwrap();
    chunk.glosses[0].text.push('!');
    assert!(
        bad.validate().is_err(),
        "a gloss must quote the exact original span"
    );
}

#[test]
fn reader_content_rejects_duplicate_identity_unknown_fields_and_catalog_versions() {
    let original = load_reader_anthology().unwrap();
    let mut bad = original.clone();
    bad.documents.push(bad.documents[0].clone());
    assert!(bad.validate().is_err());
    let mut bad = original.clone();
    bad.documents.clear();
    assert!(bad.validate().is_err());
    let mut bad = original.clone();
    bad.schema_version = 999;
    assert!(matches!(
        bad.validate(),
        Err(ContentError::UnsupportedSchema(999))
    ));
    let mut missing_rights = original.clone();
    missing_rights.documents[3].source.rights.clear();
    assert!(missing_rights.validate().is_err());
    let mut missing_help_origin = original.clone();
    missing_help_origin.documents[0].help_provenance.clear();
    assert!(missing_help_origin.validate().is_err());
    let mut value = serde_json::to_value(&original).unwrap();
    value["unreviewed_extra_schema"] = true.into();
    assert!(matches!(
        ReaderAnthology::from_json(&value.to_string()),
        Err(ContentError::Json(_))
    ));
    let mut value = serde_json::to_value(original).unwrap();
    value["documents"][0]["source"]["translation"] =
        "An AI translation is not source metadata".into();
    assert!(matches!(
        ReaderAnthology::from_json(&value.to_string()),
        Err(ContentError::Json(_))
    ));
}

#[test]
fn reader_content_v1_definitions_are_immutable_including_source_and_ai_help() {
    fn ordered(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(fields) => {
                let mut entries: Vec<_> = fields.into_iter().collect();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                let mut result = serde_json::Map::new();
                for (key, value) in entries {
                    result.insert(key, ordered(value));
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(ordered).collect())
            }
            other => other,
        }
    }
    let anthology = load_reader_anthology().unwrap();
    // Frozen after two AI editorial reviews on 2026-11-06. These digest the full
    // definition, not just original text: same id/version with changed help would
    // otherwise emit unequal immutable ReaderDocument facts under the same UUID.
    for (id, expected) in [
        (
            "ru.reader.echo",
            "92fb540170d418df993ddd519c1cd14d0aaf94f735db789671cf130c1df94f99",
        ),
        (
            "ru.reader.no_largest",
            "1c1b240478eb4675f5e04cd50fb2795cc4fcbd619c61d7d91c6c36898888c1d6",
        ),
        (
            "ru.reader.words_and_evidence",
            "3796bf1cea51aa54cf15d68b84ab20bc697e1ced08f760b7134aabee8abe960d",
        ),
        (
            "ru.reader.chekhov_student_opening",
            "d0639169bb60e39a6388f196727d62bd538da166a0ac34c8de252cf9933b735f",
        ),
    ] {
        let definition = anthology.definition(id, "1").unwrap();
        let canonical =
            serde_json::to_string(&ordered(serde_json::to_value(definition).unwrap())).unwrap();
        assert_eq!(
            rufren_core::reader_text_sha256(&canonical),
            expected,
            "publish an explicitly new version for changed Reader definition {id}"
        );
    }
}
