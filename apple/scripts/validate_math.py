#!/usr/bin/env python3
"""Validate the original mathematics shelf; no third-party dependencies.

Coverage is conservative, exact surface-form exposure in the course, not inferred
lemma mastery. A form is available only once a lexicon entry containing it has
been presented with meaning, by the end of after_page (course array order).
Earlier shelf encounters are reported separately, never treated as course words.
"""
import argparse
import json
import re
import sys
from pathlib import Path

import validate_library as library

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / 'Drussky/Resources/math.json'
COURSE = ROOT / 'Drussky/Resources/course.json'
ACUTE = '\u0301'
WORD_PART = r'[А-Яа-яЁё]+(?:\u0301[А-Яа-яЁё]*)*'
WORD = re.compile(WORD_PART + '(?:-' + WORD_PART + ')*')
VOWELS = set('аеёиоуыэюя')
# Small editorial regression list, not a general Russian stress dictionary.
# Both readings of числа are legitimate; context still needs human review.
AUDITED_STRESS = {
    bare_form: set(marked.split()) for bare_form, marked in (
        ('множество', 'мно́жество'), ('множества', 'мно́жества'),
        ('число', 'число́'), ('числа', 'чи́сла числа́'), ('чисел', 'чи́сел'),
        ('точка', 'то́чка'), ('точек', 'то́чек'),
        ('равен', 'ра́вен'), ('равна', 'равна́'), ('равно', 'равно́'), ('равны', 'равны́'),
        ('определена', 'определена́'), ('определено', 'определено́'),
        ('замкнуто', 'за́мкнуто'), ('или', 'и́ли'), ('положим', 'поло́жим'),
        ('получим', 'полу́чим'), ('получили', 'получи́ли'), ('найден', 'на́йден'),
        ('дирихле', 'дирихле́'), ('появляется', 'появля́ется'),
        ('биекцией', 'бие́кцией'), ('биекции', 'бие́кции'),
    )
}


def tokens(text):
    return WORD.findall(text)


def bare(text):
    return text.replace(ACUTE, '').lower()


def course_exposure(course):
    """Mirror meaning-presentation events in validate_course, limited to lexicon.

    Whole multiword entries must be presented before their tokens count. Recall
    prompts, English explanations, and distractors do not introduce known words.
    """
    entries = {tuple(bare(w) for w in tokens(e['ru'])) for e in course['lexicon']}
    entries.discard(())
    letters = {e['upper']: e for e in course['letters']}
    known, pages = set(), {}

    def show(text, inside=False):
        seq = tuple(bare(w) for w in tokens(text))
        if seq in entries:
            known.update(seq)
        if inside:
            for entry in entries:
                if any(seq[i:i + len(entry)] == entry for i in range(len(seq))):
                    known.update(entry)

    for unit in course['units']:
        for page in unit['pages']:
            for ex in page['exercises']:
                kind = ex['kind']
                if kind == 'meet':
                    show(letters.get(ex['letter'], {}).get('example', ''))
                elif kind in ('trace', 'copy', 'read_aloud'):
                    show(ex['text'])
                elif kind == 'listen_pick':
                    show(ex['say'])
                elif kind == 'explain':
                    for example in ex.get('examples', []):
                        show(example['ru'], inside=True)
                elif kind == 'passage':
                    show(ex['text'], inside=True)
                    for word in ex.get('glosses', {}):
                        show(word)
                elif kind == 'build_word':
                    show(ex['answer'])
                elif kind == 'family_guess':
                    show(ex['word'])
            pages[page['id']] = set(known)
    return pages


def russian_strings(text):
    """All learner-facing Russian including Russian quoted in English notes.

    Dictionary keys deliberately carry no stress. Symbols are notation, not prose.
    """
    for field in ('title', 'text', 'en', 'summary_en'):
        if isinstance(text.get(field), str):
            yield field, text[field]
    for field in ('how_read', 'notation', 'questions'):
        entries = text.get(field, [])
        if not isinstance(entries, list):
            continue
        for i, entry in enumerate(entries):
            if not isinstance(entry, dict):
                continue
            for key, value in entry.items():
                if key in ('symbol', 'answer', 'kind'):
                    continue
                if isinstance(value, str):
                    yield f'{field}[{i}].{key}', value
                elif isinstance(value, list):
                    for j, option in enumerate(value):
                        if isinstance(option, str):
                            yield f'{field}[{i}].{key}[{j}]', option


def stress_errors(value):
    errors = []
    for word in tokens(value):
        lower = word.lower()
        if bare(lower) in AUDITED_STRESS and lower not in AUDITED_STRESS[bare(lower)]:
            errors.append(f'{word}: expected ' + ' or '.join(sorted(AUDITED_STRESS[bare(lower)])))
        syllables = sum(c in VOWELS for c in lower)
        if 'ё' in lower:
            if ACUTE in lower:
                errors.append(f'{word}: ё must not carry or share an acute')
        elif syllables > 1 and lower.count(ACUTE) != 1:
            errors.append(f'{word}: needs exactly one acute')
        elif lower.count(ACUTE) > 1:
            errors.append(f'{word}: multiple acutes')
        for i, char in enumerate(lower):
            if char == ACUTE and (not i or lower[i-1] not in VOWELS - {'ё'}):
                errors.append(f'{word}: acute is not after a vowel')
    # Reject orphan accents too, rather than silently dropping them in tokenizing.
    for i, char in enumerate(value):
        if char == ACUTE and (not i or value[i-1].lower() not in VOWELS - {'ё'}):
            errors.append('orphan acute or acute on a non-vowel/ё')
    return sorted(set(errors))


def validate(path=DEFAULT, course_path=COURSE):
    errors, notes, rows = [], [], []
    try:
        data = json.loads(path.read_text())
        course = json.loads(course_path.read_text())
    except (OSError, ValueError) as e:
        return errors + [str(e)], notes, rows
    if not isinstance(data, dict) or not isinstance(data.get('texts'), list):
        return ['collection must be an object with a texts array'], notes, rows
    if not all(isinstance(t, dict) for t in data['texts']):
        return ['every text must be an object'], notes, rows
    if not isinstance(data.get('sentences', []), list) or not all(isinstance(s, dict) for s in data.get('sentences', [])):
        return ['sentences must be an array of objects'], notes, rows
    library.check(path.parent, True, errors, notes, filename=path.name)
    if set(data) - {'version', 'collection', 'texts', 'sentences'}:
        errors.append('unknown top-level fields: ' + ', '.join(sorted(set(data) - {'version', 'collection', 'texts', 'sentences'})))
    pages = course_exposure(course)
    order = {page: i for i, page in enumerate(pages)}
    collection = data.get('collection', {})
    if not isinstance(collection, dict) or collection.get('id') != 'mathematics':
        errors.append('collection.id must be mathematics')
    else:
        for key in ('title', 'title_en', 'description_en'):
            if not isinstance(collection.get(key), str) or not collection[key].strip():
                errors.append(f'collection.{key} must be a nonempty string')
        errors.extend('collection.title: ' + e for e in stress_errors(collection.get('title', '')))
    texts = data.get('texts', [])
    if len(texts) != 24:
        errors.append(f'expected 24 rungs, found {len(texts)}')
    prior, last_page = set(), -1
    for rung, text in enumerate(texts, 1):
        tag = text.get('id', f'rung {rung}')
        fail = lambda message: errors.append(f'{tag}: {message}')
        if type(text.get('rung')) is not int or text['rung'] != rung:
            fail('rung must match 1-based collection order')
        if text.get('topic') != 'math':
            fail('topic must be math')
        page = text.get('after_page')
        if not isinstance(page, str) or page not in pages:
            fail(f'after_page {page!r} is not a course page id')
            known = set()
        else:
            known = pages[page]
            if order[page] < last_page:
                fail('after_page must not move backwards through the course')
            last_page = order[page]
        prose = text.get('text', '')
        if not isinstance(prose, str):
            fail('text must be a string')
            continue
        words = [bare(w) for w in tokens(prose)]
        # Count Russian words, not Latin variables, numerals, or isolated symbols.
        if not 60 <= len(words) <= 220:
            fail(f'{len(words)} Russian words; expected 60–220')
        if not isinstance(text.get('en'), str) or not text['en'].strip():
            fail('en must be a nonempty complete translation')
        if isinstance(text.get('en'), str) and text['en'].count('\n') != prose.count('\n'):
            fail('translation paragraph count must match text')
        glosses = text.get('glosses', {})
        if not isinstance(glosses, dict):
            fail('glosses must be a dictionary')
            glosses = {}
        needed = set()
        for field, value in russian_strings(text):
            needed.update(bare(w) for w in tokens(value))
            for e in stress_errors(value):
                fail(f'{field}: {e}')
        for key, value in glosses.items():
            if not re.fullmatch(r'[а-яё]+(?:-[а-яё]+)*', key):
                fail(f'gloss key {key!r} must be a single lowercase unstressed form')
            if not isinstance(value, str) or not value.strip():
                fail(f'gloss {key!r} needs an English contextual meaning')
            elif stress_errors(value):
                fail(f'gloss {key!r}: ' + '; '.join(stress_errors(value)))
            if key not in needed:
                fail(f'unused gloss {key!r}')
        missing = needed - known - set(glosses)
        if missing:
            fail('missing glosses: ' + ', '.join(sorted(missing)))
        frames = text.get('how_read', [])
        if not isinstance(frames, list) or not 3 <= len(frames) <= 5:
            fail('how_read must contain 3–5 frames')
        else:
            for frame in frames:
                if not isinstance(frame, dict) or set(frame) != {'ru', 'en', 'note'} or not all(isinstance(v, str) and v.strip() for v in frame.values()):
                    fail('each how_read frame needs nonempty ru/en/note strings')
        notation = text.get('notation', [])
        if not isinstance(notation, list) or not notation:
            fail('notation must contain at least one reading')
        else:
            for item in notation:
                if not isinstance(item, dict) or set(item) != {'symbol', 'ru', 'en'} or not all(isinstance(v, str) and v.strip() for v in item.values()):
                    fail('each notation item needs nonempty symbol/ru/en strings')
        questions = text.get('questions', [])
        if not isinstance(questions, list) or len(questions) != 3:
            fail('exactly three questions required')
        else:
            if [q.get('kind') if isinstance(q, dict) else None for q in questions] != ['gist', 'detail', 'inference']:
                fail('questions must be gist/detail/inference, in order')
            for q in questions:
                if not isinstance(q, dict):
                    fail('question must be an object')
                    continue
                for key in ('prompt', 'explanation'):
                    if not isinstance(q.get(key), str) or not q[key].strip():
                        fail(f'question.{key} must be a nonempty string')
                options = q.get('options', [])
                if not isinstance(options, list) or len(options) != 3 or not all(isinstance(o, str) and o.strip() for o in options):
                    fail('question needs three nonempty options')
                    options = []
                elif len(set(options)) != 3:
                    fail('question options must be distinct')
                if type(q.get('answer')) is not int or not 0 <= q['answer'] < len(options):
                    fail('answer must be a valid zero-based integer option index')
        refs = text.get('references', [])
        if not isinstance(refs, list):
            fail('references must be an array')
        else:
            for ref in refs:
                if (not isinstance(ref, dict) or set(ref) != {'url', 'note'}
                        or not isinstance(ref.get('url'), str) or not ref['url'].startswith('https://')
                        or not isinstance(ref.get('note'), str) or not ref['note'].strip()):
                    fail('each reference needs an HTTPS url and a nonempty note')
        types = set(words)
        count_known = sum(w in known for w in words)
        supported = sum(w in known or w in prior for w in words)
        rows.append((rung, page, len(words), len(types), count_known,
                     100 * count_known / max(1, len(words)),
                     100 * supported / max(1, len(words)), len(types-known-prior), len(glosses), len(missing)))
        prior.update(types)
    return errors, notes, rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', nargs='?', type=Path, default=DEFAULT)
    parser.add_argument('--course', type=Path, default=COURSE)
    args = parser.parse_args()
    errors, notes, rows = validate(args.path, args.course)
    for note in notes:
        print(note)
    print('Rung after_page words types course_tokens course% +prior% new_types glosses missing')
    for row in rows:
        print(f'{row[0]:02d} {str(row[1]):11} {row[2]:5} {row[3]:5} {row[4]:13} {row[5]:6.1f} {row[6]:7.1f} {row[7]:9} {row[8]:7} {row[9]:7}')
    print('course% = exact forms presented with meaning by after_page; +prior% adds earlier shelf encounters, not mastery.')
    print('new_types = passage forms absent from both; glosses cover all learner-facing Russian, including frames and notation.')
    for error in errors:
        print('ERROR', error)
    print('clean' if not errors else f'{len(errors)} errors')
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
