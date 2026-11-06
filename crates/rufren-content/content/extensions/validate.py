#!/usr/bin/env python3
"""Read-only editorial check of the extension against the embedded pack.

The Rust ContentPack validator remains authoritative for runtime loading. This
check is deliberately runnable without competing for the workspace build lock.
"""
import json
from pathlib import Path
import re
import unicodedata

HERE = Path(__file__).resolve().parent
BASE = HERE.parents[1] / 'data' / 'beginner-v1.json'
EXTENSION = HERE / 'conversation-to-ideas-v1.json'
SECTIONS = ('units', 'lessons', 'targets', 'phrases', 'exercises', 'sources')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate():
    base = json.loads(BASE.read_text())
    extension = json.loads(EXTENSION.read_text())
    require(extension['extension_schema_version'] == 1, 'unsupported extension format')
    require(extension['base_pack_id'] == base['id'], 'wrong base pack identity')
    require(extension['required_base_revision'] in [base['revision'], *base['previous_revisions']], 'required base revision absent from ancestry')
    combined = {}
    for section in SECTIONS:
        original = {item['id']: item for item in base[section]}
        require(len(original) == len(base[section]), f'duplicate base {section}')
        additions = extension[section]
        require(len({x['id'] for x in additions}) == len(additions), f'duplicate extension {section}')
        # Also works after this exact extension has been incorporated in the pack.
        for item in additions:
            if item['id'] in original:
                require(original[item['id']] == item, f'conflicting integrated item {item["id"]}')
            else:
                original[item['id']] = item
            require(set(item) == set(base[section][0]), f'unknown/missing {section} field in {item["id"]}')
        combined[section] = original
    all_ids = [item_id for section in SECTIONS for item_id in combined[section]]
    require(len(set(all_ids)) == len(all_ids), 'ID reused across content kinds')
    require(all(x.startswith('ru.') for x in all_ids), 'non-Russian content identity')
    lessons, phrases, exercises, targets = [combined[x] for x in ('lessons', 'phrases', 'exercises', 'targets')]
    covered_lessons, covered_exercises, covered_phrases, preceding = set(), set(), set(), set()
    for unit in combined['units'].values():
        for item_id in unit['lesson_ids']:
            require(item_id not in covered_lessons, f'duplicate unit membership {item_id}')
            require(lessons[item_id]['unit_id'] == unit['id'], f'bad unit membership {item_id}')
            covered_lessons.add(item_id)
    for lesson in lessons.values():
        key = lesson['id']
        require(set(lesson['prerequisites']) <= preceding, f'nonpreceding lesson prerequisite {key}')
        preceding.add(key)
        require(key in covered_lessons, f'orphan lesson {key}')
        require(lesson['phrase_ids'] and lesson['exercise_ids'], f'empty lesson {key}')
        require(set(lesson['target_ids']) <= targets.keys(), f'unknown lesson target {key}')
        for item_id in lesson['phrase_ids']:
            require(item_id in phrases, f'missing phrase {item_id}')
            covered_phrases.add(item_id)
        for item_id in lesson['exercise_ids']:
            require(item_id not in covered_exercises, f'duplicate exercise membership {item_id}')
            require(exercises[item_id]['lesson_id'] == key, f'wrong exercise lesson {item_id}')
            covered_exercises.add(item_id)
        for turn in lesson['dialogue']['turns']:
            require(turn['speaker'] and turn['phrase_id'] in lesson['phrase_ids'], f'bad dialogue {key}')
        for explanation in lesson['explanations']:
            require(set(explanation['source_ids']) <= combined['sources'].keys(), f'unknown source {key}')
    require(covered_phrases == phrases.keys(), 'orphan phrases')
    require(covered_exercises == exercises.keys(), 'orphan exercises')
    for phrase in phrases.values():
        key = phrase['id']
        require(phrase['chunks'] and ' '.join(phrase['chunks']) == phrase['stressed'], f'chunks differ {key}')
        require(unicodedata.normalize('NFC', phrase['stressed'].replace('\u0301', '')) == unicodedata.normalize('NFC', phrase['russian']), f'stress changes spelling {key}')
        require(set(phrase['target_ids']) <= targets.keys(), f'unknown phrase target {key}')
    def visit(key, active, done):
        require(key in targets, f'unknown target dependency {key}')
        if key in done:
            return
        require(key not in active, f'target cycle {key}')
        active.add(key)
        for dep in targets[key]['prerequisites']:
            visit(dep, active, done)
        active.remove(key)
        done.add(key)
    done = set()
    for key in targets:
        visit(key, set(), done)
    for exercise in exercises.values():
        key = exercise['id']
        require(exercise['reference_phrase_id'] in lessons[exercise['lesson_id']]['phrase_ids'], f'bad hidden anchor {key}')
        require(set(exercise['stimulus_phrase_ids']) <= phrases.keys(), f'unknown stimulus {key}')
        require(set(exercise['target_ids']) <= targets.keys() and exercise['target_ids'], f'unknown or empty exercise target {key}')
        kind, answer = exercise['kind'], exercise['answer']
        if kind in ('written_retrieval', 'spoken_retrieval', 'composition'):
            require(not exercise['stimulus_phrase_ids'], f'production leaks written model {key}')
        if kind in ('listening_comprehension', 'dictation'):
            require(exercise['requires_audio'] and exercise['stimulus_phrase_ids'], f'hearing task lacks audio {key}')
        if answer['type'] == 'choice':
            ids = [x['id'] for x in exercise['options']]
            require(len(ids) >= 2 and len(set(ids)) == len(ids), f'bad choices {key}')
            require(answer['correct_option'] in ids, f'missing correct choice {key}')
        elif answer['type'] == 'exact':
            require(answer['variants'] and all(x.strip() for x in answer['variants']), f'empty exact answer {key}')
        else:
            require(answer['type'] == 'coach_review' and answer['example'] and answer['criteria'], f'bad contextual review {key}')
    listen_positions = []
    vowels = 'аеёиоуыэюяАЕЁИОУЫЭЮЯ'
    for phrase in extension['phrases']:
        require(phrase['editorial_status'] == 'generated_draft' and phrase['audio'] == 'pending_generated_speech', f'false provenance {phrase["id"]}')
        for word in re.findall(r'[А-Яа-яЁё\u0301]+', phrase['stressed']):
            if sum(c in vowels for c in word) > 1:
                require('\u0301' in word or 'ё' in word.lower(), f'missing stress scaffold: {word} in {phrase["id"]}')
            for i, char in enumerate(word):
                if char == '\u0301':
                    require(i > 0 and word[i - 1] in vowels, f'stress on nonvowel: {word}')
    for exercise in extension['exercises']:
        if exercise['kind'] == 'listening_comprehension':
            require(not any(re.search('[А-Яа-яЁё]', option['text']) for option in exercise['options']), f'listening option leaks script {exercise["id"]}')
            listen_positions.append([x['id'] for x in exercise['options']].index(exercise['answer']['correct_option']))
        if exercise['response_mode'] == 'speech':
            require(exercise['answer']['type'] == 'coach_review', f'speech wrongly exact-graded {exercise["id"]}')
    require(listen_positions.count(0) == listen_positions.count(1), 'unbalanced listening answer positions')
    for lesson in extension['lessons']:
        kinds = {exercises[key]['kind'] for key in lesson['exercise_ids']}
        require({'reading_comprehension', 'listening_comprehension', 'spoken_retrieval', 'written_retrieval', 'composition', 'dictation', 'contrast'} <= kinds, f'missing modality in {lesson["id"]}')
    print('Editorial structural checks passed; Russian naturalness and acoustic quality still require judgment.')
    print('Extension:', ', '.join(f'{len(extension[s])} {s}' for s in SECTIONS))
    print('Combined:', ', '.join(f'{len(combined[s])} {s}' for s in SECTIONS))


if __name__ == '__main__':
    validate()
