#!/usr/bin/env python3
"""Prepare fixed first takes from reviewed requests, outside the runtime assets.

Run through run-bounded.py with the pinned Silero Python. No network, model
downloads, fallback, seed search or installation. Existing takes are immutable.
"""
import argparse
import array
import base64
import contextlib
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import sys
import time
import wave

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts/speech'))
sys.dont_write_bytecode = True
from silero_frontend import synthesis_input
import silero_gateway as gateway


def digest(data):
    return hashlib.sha256(data).hexdigest()


def atomic_json(path, value):
    temporary = path.with_suffix('.pending')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temporary.replace(path)


def deadline(signum, frame):
    raise TimeoutError('One preparation request exceeded its fixed deadline')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ids', type=Path, help='JSON array of exact requested IDs')
    parser.add_argument('output', type=Path)
    parser.add_argument('--resume', action='store_true', help='Verify and skip saved takes; never regenerate them')
    args = parser.parse_args()
    bindings_path = ROOT / 'crates/rufren-content/content/extensions/alphabet-construction-bindings-v1.json'
    bindings = json.loads(bindings_path.read_text())
    requests = {item['id']: item for item in bindings['audio_requests']}
    ids = json.loads(args.ids.read_text())
    assert isinstance(ids, list) and 0 < len(ids) <= 92 and len(set(ids)) == len(ids)
    assert all(isinstance(i, str) and re.fullmatch(r'foundation-[a-z0-9-]+', i) and i in requests for i in ids)
    output = args.output.resolve()
    if output.exists() and not args.resume:
        raise ValueError('Output exists; preserve it or explicitly verify/resume')
    output.mkdir(parents=True, exist_ok=True)
    recipe = {'binding_sha256': digest(bindings_path.read_bytes()), 'script_sha256': digest(Path(__file__).read_bytes()), 'ids': ids, 'source_revision': bindings['source_pack_revision'], 'seed': 42, 'voice': gateway.VOICE, 'model': gateway.MODEL_IDENTITY, 'policy': 'one fixed first take per item; no source audio edits; no quality certification'}
    recipe_path = output / 'recipe.json'
    if recipe_path.exists():
        assert json.loads(recipe_path.read_text()) == recipe, 'Resume recipe changed'
    else:
        atomic_json(recipe_path, recipe)
    gateway.check_installation()
    signal.signal(signal.SIGALRM, deadline)
    os.nice(10)
    with (output / 'model-diagnostics.log').open('a') as diagnostics:
        with contextlib.redirect_stdout(diagnostics), contextlib.redirect_stderr(diagnostics):
            loaded = gateway.load_model()
        for identity in ids:
            item = requests[identity]
            record_path, wav_path = output / (identity + '.json'), output / (identity + '.wav')
            if record_path.exists() and args.resume:
                record = json.loads(record_path.read_text())
                assert record['source_response']['sha256'] == digest(wav_path.read_bytes())
                assert record['source_response']['source_text'] == item['marked_stress_request']
                continue
            if record_path.exists() or wav_path.exists() or (output / (identity + '.error.json')).exists():
                raise ValueError('Unreviewed prior attempt exists: ' + identity)
            text = item['marked_stress_request']
            assert text.replace('\u0301', '') == item['ordinary_spelling']
            transformed = synthesis_input(text)
            request = gateway.validate({'schema': gateway.SCHEMA, 'request_id': identity, 'revision_id': recipe['source_revision'], 'text': transformed['synthesis_text'], 'source_text': text, 'source_text_sha256': digest(text.encode()), 'mode': 'explicit_stress' if transformed['transform'] == 'pedagogical_acute_to_silero_plus' else 'automatic'})
            started = time.monotonic()
            try:
                signal.alarm(45)
                with contextlib.redirect_stdout(diagnostics), contextlib.redirect_stderr(diagnostics):
                    response, signals = gateway.synthesize(request, loader=lambda: loaded)
                signal.alarm(0)
                wav = base64.b64decode(response.pop('wav_base64'), validate=True)
                assert digest(wav) == response['sha256']
                with wav_path.open('xb') as target:
                    target.write(wav)
                with wave.open(io.BytesIO(wav)) as reader:
                    assert (reader.getnchannels(), reader.getsampwidth(), reader.getframerate()) == (1, 2, 24000)
                    assert reader.getnframes() == response['frames']
                    pcm = array.array('h', reader.readframes(reader.getnframes()))
                assert .1 <= len(pcm) / 24000 <= 12 and max(abs(v) for v in pcm) > 100
                assert signals['clipped_samples'] == 0
                record = {'id': identity, 'file': wav_path.name, 'spoken': item['ordinary_spelling'], 'display_text': text, 'teaching_span': item['purpose'], 'encoding': 'RIFF/WAVE PCM16 little-endian mono', 'byte_length': len(wav), 'source_response': response, 'measurements': {**signals, 'duration_seconds': len(pcm) / 24000, 'generation_seconds': round(time.monotonic() - started, 4)}, 'runtime': {'runtime': 'torch==' + gateway.TORCH_VERSION, 'device': 'cpu', 'threads': 2, 'model_license': gateway.LICENSE, 'model_source_revision': gateway.SOURCE_REVISION}, 'transforms': [], 'review': {'status': 'prepared; lexical and acoustic review pending', 'requested_checks': item['auditory_checks']}, 'prepared_at': datetime.now(timezone.utc).isoformat()}
                atomic_json(record_path, record)
                print(json.dumps({'id': identity, 'seconds': round(len(pcm) / 24000, 3), 'sha256': response['sha256']}), flush=True)
            except BaseException as error:
                signal.alarm(0)
                atomic_json(output / (identity + '.error.json'), {'id': identity, 'kind': type(error).__name__, 'input': request, 'original_retained': wav_path.exists()})
                raise
    records = [json.loads((output / (identity + '.json')).read_text()) for identity in ids]
    atomic_json(output / 'manifest.json', {'format': 'rufren-foundation-audio-v1', 'status': 'prepared first takes pending review; not installed', 'recipe': recipe, 'assets': records})


if __name__ == '__main__':
    main()
