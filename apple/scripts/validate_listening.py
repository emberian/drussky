#!/usr/bin/env python3
"""Validate the authored listening shelf and its reserved weekly messages.

    python3 apple/scripts/validate_listening.py [listening.json] [--write-inventory]

Stdlib only. Course order is array order, not the numeric unit label. Exact presented
surface forms count as taught; inflections are not inferred. Explicit lexicon
first_page metadata, when present, overrides the snapshot for that entry. Titles
are shelf navigation, not taught vocabulary. All other Russian content needs exact
surface-form coverage or a contextual gloss. See DEPUTY_REPORT.md for the wire schema.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import re
import sys

import validate_course
from validate_series import build_course_snapshots, key, words, ACUTE

ROOT = Path(__file__).resolve().parents[2]
RESOURCE = ROOT / 'apple/Drussky/Resources'
DEFAULT = RESOURCE / 'listening.json'
INVENTORY = ROOT / 'research/artifacts/listening-spoken.txt'
KINDS = {'pick_object', 'who_needs', 'follow', 'question_or_statement',
         'message', 'correction', 'disagreement'}
AUDITED_STRESS = {'нужна': 'нужна́', 'права': 'права́', 'потом': 'пото́м'}
COMMON = {'id', 'kind', 'en', 'after_page', 'voice_hint', 'glosses', 'crucial_span', 'why'}
EXTRA = {
    'pick_object': {'ru', 'question', 'options', 'answer'},
    'who_needs': {'lines', 'question', 'options', 'answer'},
    'follow': {'ru', 'question', 'scene', 'options', 'outcomes', 'answer'},
    'question_or_statement': {'forms', 'played', 'question', 'options', 'answer'},
    'message': {'ru', 'questions'},
    'correction': {'ru', 'question', 'options', 'answer'},
    'disagreement': {'lines', 'question', 'options', 'answer'},
}


def spoken_strings(item):
    """Exact playback candidates, including post-answer span and label audition.

    All forms are returned for intonation comparisons, not just the selected form.
    IDs/English/questions/titles are never spoken. Dedupe at export, not by audio_key:
    punctuation and authored stress must survive for the audio lane to inspect.
    """
    if isinstance(item.get('ru'), str):
        yield item['ru']
    for field in ('lines', 'forms', 'scene'):
        for part in item.get(field, []):
            if isinstance(part, dict) and isinstance(part.get('ru'), str):
                yield part['ru']
    if item.get('kind') == 'pick_object':
        yield from (x for x in item.get('options', []) if isinstance(x, str))
    if isinstance(item.get('crucial_span'), str):
        yield item['crucial_span']
    for q in item.get('questions', []):
        if isinstance(q, dict) and isinstance(q.get('crucial_span'), str):
            yield q['crucial_span']


def transcript_strings(item):
    if item.get('kind') == 'question_or_statement':
        return [x.get('ru', '') for x in item.get('forms', []) if isinstance(x, dict)]
    if item.get('kind') in ('who_needs', 'disagreement'):
        return [x.get('ru', '') for x in item.get('lines', []) if isinstance(x, dict)]
    return [item.get('ru', '')]


def course_vocabulary(course):
    index, letters, snapshots, _ = build_course_snapshots(course)
    explicit = []
    for entry in course.get('lexicon', []):
        page = entry.get('first_page')
        if page is not None:
            if page not in index:
                raise ValueError(f"lexicon {entry.get('ru')!r}: unknown first_page {page!r}")
            explicit.append((index[page], {key(w) for w in words(entry['ru'])}))
    for page, seen in snapshots.items():
        for first, forms in explicit:
            if first <= index[page]:
                seen.update(forms)
            else:
                seen.difference_update(forms)
    return index, letters, snapshots


def validate(data, course):
    errors, reports = [], []
    def fail(where, reason):
        errors.append(f'{where}: {reason}')
    def fields(obj, expected, where):
        if set(obj) != expected:
            fail(where, f'fields: missing {sorted(expected-set(obj))}, unexpected {sorted(set(obj)-expected)}')
    def text(value, where, english=False):
        if not isinstance(value, str) or not value.strip():
            fail(where, 'expected nonempty string')
            return False
        if '\n' in value and not where.endswith('.en'):
            fail(where, 'playback and labels must be single-line strings')
        if where.endswith('.ru') and not words(value):
            fail(where, 'expected Russian text')
        if english and words(value):
            fail(where, 'expected English, without Cyrillic')
        return True
    def stress(value, where):
        if not isinstance(value, str):
            return
        # Reuse the course checker exactly; isolate its legacy module-level error sink.
        start = len(validate_course.errors)
        validate_course.check_stress(where, value)
        errors.extend(validate_course.errors[start:])
        del validate_course.errors[start:]
        for token in words(value):
            expected = AUDITED_STRESS.get(key(token))
            if expected and token.lower() != expected:
                fail(where, f'audited stress: expected {expected!r}, got {token!r}')
    def all_stress(obj, where):
        if isinstance(obj, dict):
            for k, v in obj.items():
                # Unstressed dictionary keys are intentionally excluded, values are not.
                if k == 'glosses' and isinstance(v, dict):
                    for g, definition in v.items():
                        stress(definition, f'{where}.glosses[{g}]')
                else:
                    all_stress(v, f'{where}.{k}')
        elif isinstance(obj, list):
            for j, v in enumerate(obj):
                all_stress(v, f'{where}[{j}]')
        elif isinstance(obj, str):
            stress(obj, where)
    def choices(obj, where, lo=3, hi=4, english=True):
        opts = obj.get('options')
        if not isinstance(opts, list) or not lo <= len(opts) <= hi:
            fail(where, f'options must have {lo}..{hi} entries')
            return []
        for j, opt in enumerate(opts):
            text(opt, f'{where}.options[{j}]', english)
        if len({json.dumps(x, sort_keys=True) for x in opts}) != len(opts):
            fail(where, 'duplicate choices')
        index_answer(obj.get('answer'), len(opts), where)
        return opts
    def index_answer(answer, count, where):
        if type(answer) is not int or not 0 <= answer < count:
            fail(where, f'answer index {answer!r} outside 0..{count-1}')
    def span_check(span, transcripts, where):
        if text(span, where) and not any(isinstance(t, str) and span in t for t in transcripts):
            fail(where, 'must be an exact substring of a single spoken turn/form')
    def russian_strings(obj):
        if isinstance(obj, dict):
            for k, v in obj.items():
                if k != 'glosses':
                    yield from russian_strings(v)
        elif isinstance(obj, list):
            for v in obj:
                yield from russian_strings(v)
        elif isinstance(obj, str) and words(obj):
            yield obj

    if not isinstance(data, dict):
        return ['root: expected object'], []
    fields(data, {'version', 'sets', 'weekly_samples'}, 'root')
    if type(data.get('version')) is not int or data['version'] != 1:
        fail('root', 'version must be integer 1')
    try:
        page_index, letters, known_by_page = course_vocabulary(course)
    except (ValueError, KeyError, TypeError) as exc:
        return [f'course: {exc}'], []
    all_stress(data, 'root')
    ids, practice_audio, weekly_audio = set(), set(), set()
    course_audio = {validate_course.key(s) for s in validate_course.spoken_strings(course)}
    def identifier(value, where):
        if not isinstance(value, str) or not re.fullmatch(r'[a-z][a-z0-9-]+', value):
            fail(where, 'expected lowercase ASCII kebab-case id')
        elif value in ids:
            fail(where, f'duplicate id {value}')
        else:
            ids.add(value)
    def page(value, where):
        if not isinstance(value, str) or value not in page_index:
            fail(where, f'unknown after_page {value!r}')
            return 0
        return page_index[value]

    for bank, count in [('sets', 12), ('weekly_samples', 6)]:
        groups = data.get(bank)
        if not isinstance(groups, list):
            fail(bank, 'expected array')
            continue
        if len(groups) != count:
            fail(bank, f'expected {count} sets, got {len(groups)}')
        prev_group = 0
        for gi, group in enumerate(groups):
            loc = f'{bank}[{gi}]'
            if not isinstance(group, dict):
                fail(loc, 'expected object')
                continue
            fields(group, {'id', 'title', 'title_en', 'after_page', 'items'}, loc)
            identifier(group.get('id'), loc)
            loc = group.get('id', loc)
            text(group.get('title'), loc+'.title')
            text(group.get('title_en'), loc+'.title_en', True)
            group_page = page(group.get('after_page'), loc)
            if group_page < prev_group:
                fail(loc, 'set support pages must be nondecreasing in course array order')
            prev_group = group_page
            items = group.get('items')
            if not isinstance(items, list):
                fail(loc, 'items must be array')
                continue
            low, high = (5, 5) if bank == 'weekly_samples' else (6, 10)
            if not low <= len(items) <= high:
                fail(loc, f'expected {low}..{high} items')
            previous = group_page
            total, covered, gloss_tokens, unfamiliar = 0, 0, 0, 0
            new_forms, displayed_unknown_letters = set(), set()
            kinds = Counter()
            for j, item in enumerate(items):
                iloc = f'{loc}[{j}]'
                if not isinstance(item, dict):
                    fail(iloc, 'expected object')
                    continue
                identifier(item.get('id'), iloc)
                iloc = item.get('id', iloc)
                k = item.get('kind')
                if not isinstance(k, str) or k not in KINDS:
                    fail(iloc, f'unknown kind {k!r}')
                    continue
                fields(item, COMMON | EXTRA[k], iloc)
                kinds[k] += 1
                text(item.get('en'), iloc+'.en', True)
                text(item.get('why'), iloc+'.why')
                p = page(item.get('after_page'), iloc)
                if p < previous:
                    fail(iloc, 'after_page precedes set support page or previous item')
                previous = p
                known = known_by_page.get(item.get('after_page'), set())
                if item.get('voice_hint') not in ('any', 'unfamiliar'):
                    fail(iloc, 'voice_hint must be any or unfamiliar')
                unfamiliar += item.get('voice_hint') == 'unfamiliar'
                if bank == 'weekly_samples' and (k != 'message' or item.get('voice_hint') != 'unfamiliar'):
                    fail(iloc, 'weekly items must be unfamiliar messages')

                if k in ('pick_object', 'follow', 'message', 'correction'):
                    text(item.get('ru'), iloc+'.ru')
                if k != 'message':
                    text(item.get('question'), iloc+'.question', True)
                if k in ('pick_object', 'who_needs', 'correction', 'disagreement'):
                    opts = choices(item, iloc, english=k != 'pick_object')
                    if k == 'pick_object':
                        for opt in opts:
                            if isinstance(opt, str) and (len(words(opt)) != 1 or key(opt) not in known):
                                fail(iloc, f'object choice {opt!r} must be a single already-taught noun form')
                if k in ('who_needs', 'disagreement'):
                    lines = item.get('lines')
                    if not isinstance(lines, list) or not 2 <= len(lines) <= 4:
                        fail(iloc, 'lines must contain 2..4 turns')
                    else:
                        speakers = []
                        for n, line in enumerate(lines):
                            ll = f'{iloc}.lines[{n}]'
                            if not isinstance(line, dict):
                                fail(ll, 'expected object')
                                continue
                            fields(line, {'speaker', 'ru', 'en'}, ll)
                            if line.get('speaker') not in ('A', 'B'):
                                fail(ll, 'speaker must be A or B')
                            speakers.append(line.get('speaker'))
                            text(line.get('ru'), ll+'.ru')
                            text(line.get('en'), ll+'.en', True)
                        if set(speakers) != {'A', 'B'} or any(a == b for a, b in zip(speakers, speakers[1:])):
                            fail(iloc, 'both A and B must speak in alternating turns')
                if k == 'question_or_statement':
                    forms = item.get('forms')
                    if not isinstance(forms, list) or len(forms) != 2:
                        fail(iloc, 'forms must be [statement, question]')
                    elif all(isinstance(x, dict) for x in forms):
                        for n, form in enumerate(forms):
                            fields(form, {'ru', 'en'}, f'{iloc}.forms[{n}]')
                            text(form.get('ru'), iloc+'.forms.ru')
                            text(form.get('en'), iloc+'.forms.en', True)
                        a, b = [f.get('ru', '') for f in forms]
                        if isinstance(a, str) and isinstance(b, str):
                            if not a.endswith('.') or not b.endswith('?') or a[:-1] != b[:-1]:
                                fail(iloc, 'forms must differ only in final period/question mark')
                            if not validate_course.rising_question(b) or validate_course.audio_key(a) == validate_course.audio_key(b):
                                fail(iloc, 'question must be yes/no with a distinct audio key')
                        index_answer(item.get('played'), 2, iloc+'.played')
                        if item.get('answer') != item.get('played'):
                            fail(iloc, 'answer must equal played form index')
                        selected = item.get('played')
                        if type(selected) is int and 0 <= selected < 2:
                            if item.get('en') != forms[selected].get('en') or item.get('crucial_span') != forms[selected].get('ru'):
                                fail(iloc, 'en and crucial_span must match selected form')
                    else:
                        fail(iloc, 'each form must be object')
                    if item.get('options') != ['Statement', 'Question']:
                        fail(iloc, 'intonation options must be [Statement, Question]')
                    index_answer(item.get('answer'), 2, iloc)
                if k == 'follow':
                    scene = item.get('scene')
                    entities = {}
                    if not isinstance(scene, list) or not 3 <= len(scene) <= 5:
                        fail(iloc, 'scene must contain 3..5 named entities')
                    else:
                        for entity in scene:
                            if not isinstance(entity, dict):
                                fail(iloc, 'scene entity must be object')
                                continue
                            fields(entity, {'id', 'ru', 'en', 'role'}, iloc+'.scene')
                            eid = entity.get('id')
                            if not isinstance(eid, str) or not eid or eid in entities:
                                fail(iloc, 'scene IDs must be nonempty and locally unique')
                                continue
                            entities[eid] = entity
                            text(entity.get('ru'), iloc+'.scene.ru')
                            text(entity.get('en'), iloc+'.scene.en', True)
                            if entity.get('role') not in ('object', 'place'):
                                fail(iloc, 'scene role must be object or place')
                    labels = choices(item, iloc)
                    opts = item.get('outcomes')
                    if not isinstance(opts, list) or len(opts) != len(labels):
                        fail(iloc, 'outcomes must have one entry per option')
                    else:
                        outcomes = []
                        for opt in opts:
                            if not isinstance(opt, dict):
                                fail(iloc, 'follow outcome must be object')
                                continue
                            fields(opt, {'object', 'place'}, iloc+'.outcomes')
                            obj, place_id = opt.get('object'), opt.get('place')
                            if not isinstance(obj, str) or entities.get(obj, {}).get('role') != 'object':
                                fail(iloc, 'outcome object must reference a scene object')
                            if place_id is not None and (not isinstance(place_id, str) or entities.get(place_id, {}).get('role') != 'place'):
                                fail(iloc, 'outcome place must reference a scene place, or null for unchanged')
                            outcomes.append((str(obj), str(place_id)))
                        if len(set(outcomes)) != len(outcomes):
                            fail(iloc, 'duplicate scene outcomes')
                transcripts = transcript_strings(item)
                span_check(item.get('crucial_span'), transcripts, iloc+'.crucial_span')
                if k == 'message':
                    s = item.get('ru', '')
                    if isinstance(s, str):
                        sentences = [x for x in re.split(r'[.!?]+(?:\s+|$)', s) if x.strip()]
                        if not 2 <= len(sentences) <= 4:
                            fail(iloc, 'message must have 2..4 sentences')
                    qs = item.get('questions')
                    if not isinstance(qs, list) or not 2 <= len(qs) <= 3:
                        fail(iloc, 'message must have 2..3 questions')
                    else:
                        qkinds = set()
                        for qi, q in enumerate(qs):
                            qloc = f'{iloc}.questions[{qi}]'
                            if not isinstance(q, dict):
                                fail(qloc, 'expected object')
                                continue
                            required = {'kind', 'prompt', 'crucial_span', 'why'}
                            required |= {'response_mode', 'answer_en', 'accept_en'} if bank == 'weekly_samples' else {'options', 'answer'}
                            fields(q, required, qloc)
                            if q.get('kind') not in ('gist', 'detail'):
                                fail(qloc, 'question kind must be gist or detail')
                            else:
                                qkinds.add(q['kind'])
                            text(q.get('prompt'), qloc+'.prompt', True)
                            text(q.get('why'), qloc+'.why')
                            span_check(q.get('crucial_span'), transcripts, qloc+'.crucial_span')
                            if bank == 'weekly_samples':
                                if q.get('response_mode') != 'english_text':
                                    fail(qloc, 'weekly response_mode must be english_text')
                                text(q.get('answer_en'), qloc+'.answer_en', True)
                                acceptable = q.get('accept_en')
                                if not isinstance(acceptable, list) or not acceptable:
                                    fail(qloc, 'accept_en must contain at least one English example')
                                else:
                                    for a in acceptable:
                                        text(a, qloc+'.accept_en', True)
                            else:
                                choices(q, qloc)
                        if qkinds != {'gist', 'detail'}:
                            fail(iloc, 'message must contain both gist and detail questions')
                glosses = item.get('glosses')
                if not isinstance(glosses, dict):
                    fail(iloc, 'glosses must be object, possibly empty')
                    glosses = {}
                source_forms = {key(w) for s in russian_strings(item) for w in words(s)}
                for g, value in glosses.items():
                    if not isinstance(g, str) or ACUTE in g or g != key(g) or len(words(g)) != 1:
                        fail(iloc, f'invalid lowercase unstressed surface gloss key {g!r}')
                    text(value, iloc+f'.glosses[{g}]')
                    if g not in source_forms:
                        fail(iloc, f'gloss {g!r} never occurs in this item')
                for form in source_forms - known - set(glosses):
                    fail(iloc, f'untaught surface form {form!r} at {item.get("after_page")}, missing gloss')
                # Coverage counts the heard turns once (both variants for intonation),
                # not repeated feedback quotations or visual answer alternatives.
                tokens = [key(w) for s in transcripts if isinstance(s, str) for w in words(s)]
                total += len(tokens)
                covered += sum(w in known for w in tokens)
                gloss_tokens += sum(w not in known and w in glosses for w in tokens)
                new_forms.update(set(tokens)-known)
                if k == 'pick_object':
                    display = item.get('options', [])
                elif k == 'follow':
                    display = [x.get('ru', '') for x in item.get('scene', []) if isinstance(x, dict)]
                else:
                    display = []
                for label in display:
                    if isinstance(label, str):
                        unseen = validate_course.letters_in(label)-letters.get(item.get('after_page'), set())
                        displayed_unknown_letters.update(unseen)
                        if unseen:
                            fail(iloc, f'pre-answer label {label!r} uses letters not yet met: {sorted(unseen)}')
                if bank == 'sets':
                    practice_audio.update(validate_course.key(s) for s in transcripts if isinstance(s, str))
                    practice_audio.add(validate_course.key(' '.join(s for s in transcripts if isinstance(s, str))))
                elif isinstance(item.get('ru'), str):
                    identity = validate_course.key(item['ru'])
                    if identity in practice_audio or identity in weekly_audio or identity in course_audio:
                        fail(iloc, 'weekly message reused from practice, course, or another weekly message')
                    weekly_audio.add(identity)
            if not unfamiliar:
                fail(loc, 'at least one unfamiliar-voice item required')
            reports.append({'id':loc, 'after_page':group.get('after_page'), 'items':len(items),
                            'kinds':dict(kinds), 'tokens':total, 'known':covered,
                            'glossed':gloss_tokens, 'new_forms':len(new_forms), 'unfamiliar':unfamiliar})
    return errors, reports


def inventory(data):
    return list(dict.fromkeys(s for bank in ('sets', 'weekly_samples')
                             for group in data[bank] for item in group['items']
                             for s in spoken_strings(item)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path, nargs='?', default=DEFAULT)
    parser.add_argument('--write-inventory', action='store_true', help='write exact UTF-8 playback strings after validation')
    args = parser.parse_args()
    try:
        data = json.loads(args.path.read_text(encoding='utf-8'))
        course = json.loads((RESOURCE/'course.json').read_text(encoding='utf-8'))
        errors, reports = validate(data, course)
    except (OSError, ValueError, TypeError, KeyError) as exc:
        print(f'FAILED: {exc}', file=sys.stderr)
        return 1
    print(f'{"Set":<20} {"After":<8} {"Items":>5} {"Heard":>6} {"Course%":>8} {"Glossed":>8} {"New forms":>10} {"Unfamiliar":>10}')
    for r in reports:
        pct = 100*r['known']/max(1,r['tokens'])
        print(f'{r["id"]:<20} {r["after_page"]:<8} {r["items"]:>5} {r["tokens"]:>6} {pct:>7.1f}% {r["glossed"]:>8} {r["new_forms"]:>10} {r["unfamiliar"]:>10}')
    if errors:
        print(f'\nFAILED ({len(errors)} errors):', file=sys.stderr)
        for error in errors:
            print('  '+error, file=sys.stderr)
        return 1
    strings = inventory(data)
    rendered = '\n'.join(strings)+'\n'
    if args.write_inventory:
        INVENTORY.parent.mkdir(parents=True, exist_ok=True)
        INVENTORY.write_text(rendered, encoding='utf-8')
    elif args.path.resolve() == DEFAULT.resolve():
        if not INVENTORY.exists() or INVENTORY.read_text(encoding='utf-8') != rendered:
            print('FAILED: spoken inventory stale/missing; run --write-inventory', file=sys.stderr)
            return 1
    print(f'\nOK: schema, stress, exact-form coverage, order, answers, spans, scene references, intonation pairs, weekly isolation; {len(strings)} unique playback strings.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
