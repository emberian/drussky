#!/usr/bin/env python3
"""Build and check a draft pack without changing the embedded iPad-build source.

The output is an editorial staging artifact, never selected by include_str!.
Activation and the canonical Rust check require the workspace owner's later grant.
"""
import argparse
import hashlib
import importlib.util
import json
import sys
import tempfile
import uuid
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
EMBEDDED = CRATE / 'data' / 'beginner-v1.json'
BASE = CRATE / 'tests' / 'fixtures' / 'beginner-v1-2026-11-05.3.json'
EXTENSION = CRATE / 'content' / 'extensions' / 'conversation-to-ideas-v1.json'
DRAFT = CRATE / 'content' / 'staged' / 'beginner-v1-2026-11-05.4.json'
MANIFEST = CRATE / 'content' / 'staged' / 'merge-manifest.json'
SECTIONS = ('units', 'lessons', 'targets', 'phrases', 'exercises', 'sources')
REVISION = '2026-11-05.4'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def assemble(base, extension):
    require(base['schema_version'] == 1, 'unsupported base schema')
    require(base['revision'] == '2026-11-05.3', 'frozen .3 base changed; re-review before staging')
    require(base['id'] == extension['base_pack_id'], 'extension targets a different pack')
    require(extension['extension_schema_version'] == 1, 'unsupported extension schema')
    require(extension['required_base_revision'] == base['revision'], 'extension requires another base edition')
    require(base['editorial_status'] == extension['editorial_status'] == 'generated_draft', 'unexpected provenance')
    # JSON cloning preserves all existing item objects and prevents mutation of the base.
    combined = json.loads(json.dumps(base))
    combined['revision'] = REVISION
    combined['previous_revisions'] = list(dict.fromkeys([*base['previous_revisions'], base['revision']]))
    require(combined['previous_revisions'] == ['2026-11-05.1', '2026-11-05.2', '2026-11-05.3'], 'unexpected ancestry')
    for phrase in combined['phrases']:
        phrase['phrase_revision'] = base['revision']
        phrase['previous_phrase_revisions'] = list(base['previous_revisions'])
    used = {item['id'] for section in SECTIONS for item in base[section]}
    for section in SECTIONS:
        expected_fields = set(base[section][0])
        for item in extension[section]:
            require(set(item) == expected_fields, f'{item["id"]}: schema-v1 item fields changed')
            require(item['id'] not in used, f'duplicate identity {item["id"]}')
            used.add(item['id'])
            addition = json.loads(json.dumps(item))
            if section == 'phrases':
                addition['phrase_revision'] = REVISION
                addition['previous_phrase_revisions'] = []
            combined[section].append(addition)
        prefix = combined[section][:len(base[section])]
        if section == 'phrases':
            prefix = [phrase_payload(item) for item in prefix]
        require(prefix == base[section], f'base {section} payload was modified')
    require(set(combined) == set(base), 'extension-only root metadata leaked into runtime schema')
    require(combined['script'] == base['script'], 'alphabet changed')
    return combined


def phrase_payload(phrase):
    return {key: value for key, value in phrase.items()
            if key not in ('phrase_revision', 'previous_phrase_revisions')}


def imported_event(pack, phrase, legacy=False):
    namespace = uuid.UUID('2774e059-4afc-50c2-8e35-0e5bf8b33bed')
    stable_id = uuid.uuid5(namespace, f"{pack['id']}:{phrase['id']}")
    revision = pack['revision'] if legacy else phrase.get('phrase_revision', pack['revision'])
    previous = pack['previous_revisions'] if legacy else phrase.get('previous_phrase_revisions', pack['previous_revisions'])
    return dict(version=2, id=str(uuid.uuid5(stable_id, revision)), device=str(namespace), recorded_at_ms=0,
                fact=dict(kind='passage_revised', data=dict(
                    passage=dict(id=str(stable_id), title=phrase['translation'], russian=phrase['russian'],
                                 translation=phrase['translation'],
                                 source=f"Rufren original · editorial draft · {phrase['id']}@{revision}",
                                 coach_reviewed=False),
                    parents=sorted(str(uuid.uuid5(stable_id, predecessor)) for predecessor in previous))))


def inspect_lineage(base, combined):
    current = {phrase['id']: phrase for phrase in combined['phrases']}
    for phrase in base['phrases']:
        require(imported_event(base, phrase, legacy=True) == imported_event(combined, current[phrase['id']]),
                f"legacy immutable event changed for {phrase['id']}")
    known = {phrase['id'] for phrase in base['phrases']}
    for phrase in combined['phrases']:
        require(('phrase_revision' in phrase) == ('previous_phrase_revisions' in phrase), 'partial phrase lineage')
        parents = phrase['previous_phrase_revisions']
        require(len(parents) == len(set(parents)) and phrase['phrase_revision'] not in parents, 'invalid phrase ancestry')
        if phrase['id'] not in known:
            require(phrase['phrase_revision'] == REVISION and parents == [], 'new phrase acquired fake parents')
    return len(base['phrases'])


def inspect_graph(pack):
    lessons = {item['id']: item for item in pack['lessons']}
    encountered, waves = set(), []
    while len(encountered) < len(lessons):
        frontier = [item['id'] for item in pack['lessons']
                    if item['id'] not in encountered
                    and set(item['prerequisites']) <= encountered]
        require(frontier, 'unreachable or cyclic lesson prerequisite graph')
        waves.append(frontier)
        encountered.update(frontier)
    require(any(len(wave) > 1 for wave in waves), 'expected meaningful encounter branches')
    kinds = {kind: 0 for kind in ('reading_comprehension', 'listening_comprehension', 'written_retrieval',
                                  'spoken_retrieval', 'composition', 'dictation', 'contrast')}
    first, later = 0, 0
    for exercise in pack['exercises']:
        kinds[exercise['kind']] += 1
        if exercise['kind'] == 'listening_comprehension':
            index = next(i for i, option in enumerate(exercise['options'])
                         if option['id'] == exercise['answer']['correct_option'])
            if index == 0:
                first += 1
            else:
                later += 1
    require(first == later, 'combined listening positions became unbalanced')
    require(all(phrase['audio'] == 'pending_generated_speech' for phrase in pack['phrases']), 'AI speech default changed')
    require(all(phrase['editorial_status'] == 'generated_draft' for phrase in pack['phrases']), 'unearned review status')
    for lesson in pack['lessons']:
        require('conversation_invitation' in lesson and 'coach_activity' not in lesson, 'legacy activity key reintroduced')
    return dict(encounter_waves=waves, exercise_kinds=kinds,
                listening_correct_first=first, listening_correct_later=later)


def editorial_check(draft):
    location = CRATE / 'content' / 'extensions' / 'validate.py'
    spec = importlib.util.spec_from_file_location('rufren_extension_editorial_check', location)
    module = importlib.util.module_from_spec(spec)
    previous_bytecode_policy = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    try:
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = previous_bytecode_policy
    # Its documented integrated-item mode validates the entire combined graph and
    # the extension's stress scaffolds without rewriting either input.
    normalized = json.loads(draft.read_bytes())
    normalized['phrases'] = [phrase_payload(phrase) for phrase in normalized['phrases']]
    with tempfile.TemporaryDirectory(prefix='rufren-lineage-editorial-') as directory:
        editorial_draft = Path(directory) / 'content.json'
        editorial_draft.write_bytes(encode(normalized))
        module.BASE = editorial_draft
        module.EXTENSION = EXTENSION
        module.validate()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='verify existing staged files without writing them')
    args = parser.parse_args()
    base_bytes, extension_bytes = BASE.read_bytes(), EXTENSION.read_bytes()
    base, extension = json.loads(base_bytes), json.loads(extension_bytes)
    combined = assemble(base, extension)
    graph = inspect_graph(combined)
    preserved_events = inspect_lineage(base, combined)
    draft_bytes = encode(combined)
    manifest = dict(
        status='deterministic_candidate', pack_id=combined['id'], revision=REVISION,
        previous_revisions=combined['previous_revisions'],
        base_path=str(BASE.relative_to(CRATE)), base_sha256=sha256(base_bytes),
        extension_path=str(EXTENSION.relative_to(CRATE)), extension_sha256=sha256(extension_bytes),
        draft_path=str(DRAFT.relative_to(CRATE)), draft_sha256=sha256(draft_bytes),
        base_counts={section: len(base[section]) for section in SECTIONS},
        extension_counts={section: len(extension[section]) for section in SECTIONS},
        combined_counts={section: len(combined[section]) for section in SECTIONS},
        unchanged_base_phrase_payload_count=sum(a == phrase_payload(b) for a, b in zip(base['phrases'], combined['phrases'])),
        preserved_legacy_event_count=preserved_events,
        imported_content_event_version=2,
        canonical_rust_validation='recorded separately in coordinated workspace build logs', **graph)
    if args.check:
        require(DRAFT.read_bytes() == draft_bytes, 'staged draft differs from current deterministic merge')
        require(MANIFEST.read_bytes() == encode(manifest), 'staged manifest is stale')
    else:
        require(DRAFT.resolve() != BASE.resolve(), 'cannot write the embedded source')
        DRAFT.write_bytes(draft_bytes)
        MANIFEST.write_bytes(encode(manifest))
    editorial_check(DRAFT)
    require(BASE.read_bytes() == base_bytes, 'legacy baseline fixture changed during staging')
    require(EMBEDDED.read_bytes() in (base_bytes, draft_bytes), 'embedded pack differs from baseline and candidate')
    require(EXTENSION.read_bytes() == extension_bytes, 'extension changed during staging')
    print('Staged .4 verified; preserved legacy v2 Events:', preserved_events)
    print('Legacy .3 baseline SHA256:', sha256(base_bytes))
    print('Counts:', manifest['combined_counts'])
    print('Encounter waves:', len(graph['encounter_waves']), '; listening positions:', first_later(graph))
    print('Rust/runtime validation deferred; do not activate until the iOS build owner grants it.')


def first_later(graph):
    return f"{graph['listening_correct_first']} first / {graph['listening_correct_later']} later"


if __name__ == '__main__':
    main()
