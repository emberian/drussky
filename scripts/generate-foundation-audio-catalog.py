#!/usr/bin/env python3
"""Validate prepared offline audio and generate its single Rust catalog (no inference).

Run with --check in validation to reject a stale checked-in catalog.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import wave
import unicodedata

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/rufren-app/assets/foundations'
MODEL_SHA256 = '04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e'
MODEL_REVISION = 'e382298387332f2bf461fe6eadfde99104611b0c'
OUTPUT = ROOT / 'crates/rufren-app/src/foundation_audio_catalog.rs'


def required(obj, key):
    value = obj[key]
    if not isinstance(value, str) or not value.strip() or any(ord(c) < 32 for c in value):
        raise ValueError(f'missing/empty {key}')
    return value


def local(root, name):
    path = Path(name)
    if path.is_absolute() or '..' in path.parts or '\\' in name:
        raise ValueError(f'unsafe path: {name}')
    resolved = (root / path).resolve(strict=True)
    if not resolved.is_relative_to(root.resolve()) or not resolved.is_file():
        raise ValueError(f'unsafe/non-file path: {name}')
    return resolved


def url(obj, key):
    value = required(obj, key)
    if not value.startswith('https://') or any(c.isspace() for c in value):
        raise ValueError(f'invalid HTTPS {key}')
    return value


def digest(path, algorithm, expected):
    if not re.fullmatch('[0-9a-f]{' + str(hashlib.new(algorithm).digest_size * 2) + '}', expected):
        raise ValueError(f'invalid {algorithm}: {path.name}')
    if hashlib.new(algorithm, path.read_bytes()).hexdigest() != expected:
        raise ValueError(f'{algorithm} mismatch: {path.name}')


def record(asset, root):
    asset_id = required(asset, 'id')
    if not re.fullmatch(r'foundation-[a-z0-9]+(?:-[a-z0-9]+)*', asset_id):
        raise ValueError(f'invalid asset ID: {asset_id}')
    filename = required(asset, 'file')
    if filename != asset_id + '.wav':
        raise ValueError(f'noncanonical WAV path: {filename}')
    wav = local(root, filename)
    response = asset['source_response']
    digest(wav, 'sha256', required(response, 'sha256'))
    if wav.stat().st_size != asset['byte_length']:
        raise ValueError(f'byte length mismatch: {asset_id}')
    with wave.open(str(wav), 'rb') as audio:
        frames, rate, channels = audio.getnframes(), audio.getframerate(), audio.getnchannels()
        if audio.getsampwidth() != 2 or audio.getcomptype() != 'NONE' or frames <= 0 or rate <= 0 or channels <= 0:
            raise ValueError(f'not nonempty PCM16: {asset_id}')
        if len(audio.readframes(frames)) != frames * channels * 2:
            raise ValueError(f'truncated PCM: {asset_id}')
    if (frames, rate, channels) != (response['frames'], response['sample_rate'], response['channels']):
        raise ValueError(f'WAV identity mismatch: {asset_id}')
    if not math.isclose(asset['measurements']['duration_seconds'], frames / rate, abs_tol=1e-9):
        raise ValueError(f'duration mismatch: {asset_id}')
    provenance = asset.get('provenance', response.get('provenance'))
    if not isinstance(provenance, dict):
        raise ValueError(f'missing provenance: {asset_id}')
    transforms = asset['transforms']
    if not isinstance(transforms, list):
        raise ValueError(f'invalid transforms: {asset_id}')
    if provenance['kind'] == 'human_recording':
        original = local(root, required(provenance, 'original_file'))
        digest(original, 'sha256', required(provenance, 'original_sha256'))
        digest(original, 'sha1', required(provenance, 'commons_sha1'))
        json.loads(local(root, required(provenance, 'source_metadata')).read_text())
        url(provenance, 'original_url')
        title = 'Human recording: ' + required(asset, 'spoken')
        credit = required(provenance, 'credit')
        source = url(provenance, 'source_page')
        license_name = required(provenance, 'license')
        license_url = url(provenance, 'license_url')
        if not transforms:
            raise ValueError(f'missing decode transformation: {asset_id}')
        changes = '; '.join(required(t, 'method') for t in transforms)
    elif provenance['kind'] == 'ai_generated_speech':
        if provenance['provider'] != 'local_silero_tts' or provenance['voice'] != 'xenia' or required(provenance, 'model') not in (f'v5_6_ru@{MODEL_SHA256}', f'v5_6_ru@{MODEL_SHA256};repo={MODEL_REVISION}'):
            raise ValueError(f'unsupported synthetic provenance: {asset_id}')
        synthesis = provenance['synthesis_input']
        source_text = required(response, 'source_text')
        spoken = required(asset, 'spoken')
        if any(unicodedata.category(c).startswith('M') and c != '\u0301' for c in source_text):
            raise ValueError(f'unsupported source combining mark: {asset_id}')
        if source_text.replace('\u0301', '') != spoken:
            raise ValueError(f'synthetic source text mismatch: {asset_id}')
        transformed = []
        for c in source_text:
            if c == '\u0301':
                if not transformed or transformed[-1] not in 'аеёиоуыэюяАЕЁИОУЫЭЮЯ':
                    raise ValueError(f'misplaced pedagogical acute: {asset_id}')
                transformed.insert(len(transformed) - 1, '+')
            else:
                if c == '+':
                    raise ValueError(f'unrecorded stress syntax in source: {asset_id}')
                transformed.append(c)
        if '\u0301' in source_text:
            if (synthesis['transform'], synthesis['context'], synthesis['synthesis_text']) != ('pedagogical_acute_to_silero_plus', 'explicit_stress', ''.join(transformed)):
                raise ValueError(f'synthetic acute transformation mismatch: {asset_id}')
        elif (synthesis['transform'], synthesis['context'], synthesis['synthesis_text']) != ('verbatim', 'ordinary_text', source_text):
            raise ValueError(f'synthetic verbatim transformation mismatch: {asset_id}')
        for text_key, hash_key in [('source_text', 'source_text_sha256'), ('synthesis_text', 'synthesis_text_sha256')]:
            text = response['source_text'] if text_key == 'source_text' else required(synthesis, text_key)
            if hashlib.sha256(text.encode()).hexdigest() != required(synthesis, hash_key):
                raise ValueError(f'synthetic input hash mismatch: {asset_id}')
        required(provenance, 'prompt_version')
        required(provenance, 'validation')
        runtime = asset['runtime']
        if runtime['model_license'] != 'CC-BY-NC-SA-4.0' or required(runtime, 'model_source_revision') != MODEL_REVISION:
            raise ValueError(f'incomplete synthetic license/revision: {asset_id}')
        if transforms:
            raise ValueError(f'unhandled synthetic transformations: {asset_id}')
        title, credit = 'Synthetic reference', 'Silero v5_6_ru, Xenia voice'
        source = 'https://github.com/snakers4/silero-models'
        license_name, license_url = 'CC BY-NC-SA 4.0', 'https://creativecommons.org/licenses/by-nc-sa/4.0/'
        changes = 'Generated locally; original unmodified PCM16 WAV.'
    else:
        raise ValueError(f'unknown provenance kind: {asset_id}')
    attribution = ('Human recording: ' if provenance['kind'] == 'human_recording' else 'Synthetic reference: ') + credit + ' — ' + license_name
    return asset_id, filename, frames, rate, (title, credit, source, license_name, license_url, changes), attribution


def generate(manifest, root):
    if manifest['format'] != 'rufren-foundation-audio-v1' or not manifest['assets']:
        raise ValueError('invalid/empty manifest')
    records = [record(asset, root) for asset in manifest['assets']]
    if len({r[0] for r in records}) != len(records):
        raise ValueError('duplicate asset ID')
    quote = lambda value: json.dumps(value, ensure_ascii=False)
    lines = ['// Generated by scripts/generate-foundation-audio-catalog.py; do not edit.', 'static CATALOG: &[PreparedAudio] = &[']
    for asset_id, filename, frames, rate, credits, attribution in records:
        fields = ', '.join(f'{key}: {quote(value)}' for key, value in zip(('title', 'credit', 'source_url', 'license', 'license_url', 'changes'), credits))
        lines.append(f'    PreparedAudio {{ id: {quote(asset_id)}, bytes: include_bytes!("../assets/foundations/{filename}"), frames: {frames}, sample_rate: {rate}, credits: AudioCredits {{ {fields} }}, attribution: {quote(attribution)} }},')
    return '\n'.join(lines + ['];', ''])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    generated = generate(json.loads((ASSETS / 'manifest.json').read_text()), ASSETS)
    if args.check:
        if OUTPUT.read_text() != generated:
            raise SystemExit('foundation audio catalog is stale; regenerate it')
    else:
        OUTPUT.write_text(generated)
    print('Prepared audio catalog validated' + (' and current.' if args.check else ' and generated.'))


if __name__ == '__main__':
    main()
