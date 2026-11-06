#!/usr/bin/env python3
"""Explicit one-call-per-case acquisition; never retries or changes services."""
import base64
import datetime
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
import wave

sys.dont_write_bytecode = True
from asr_review import token_differences

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / '.local/reference-quality/slot1'
PUBLIC = ROOT / 'research/artifacts/reference-quality-slot1'
CASES = {'RQ02': 'Лук.', 'RQ03': 'Люк.', 'RQ10': 'Для каждого натурального числа существует следующее число, которое на единицу больше.'}
REUSED = {
    'hybrid-gateway-greeting-v1': '1f431043b464123eb9a023fe2cd52ecaccdca6f15f8a42d82b4328202ed2b138',
    'silero-stress-castle-v1': '3ce677ebdad80003b514c177abdb2eb7efbc7674a92acbc2abf2f80b8d51ebe4',
    'silero-stress-lock-v1': '3f0198c2415844291042e3161e445520e6d9aad7aec0c6956596a8c1e2d0ec76',
}

def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write('\n')

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        raise RuntimeError('Redirect forbidden')

def main():
    case = sys.argv[1]
    if case not in CASES: raise ValueError('Outside authorized slot')
    OUT.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    verification = []
    for name, expected in REUSED.items():
        path = ROOT / '.local/silero-tts/samples' / (name + '.wav')
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected: raise ValueError('Reused fixture changed')
        verification.append({'path': str(path.relative_to(ROOT)), 'sha256': actual})
    # Exclusive attempt marker is written before any request; reruns cannot infer.
    folder = OUT / case
    folder.mkdir()
    save(folder / 'attempt.json', {'case': case, 'started_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
         'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
         'script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'reused_verified': verification,
         'tts_attempt_ceiling': 1, 'asr_attempt_ceiling': 1})
    request = {'text': CASES[case], 'request_id': 'reference-quality-slot1-' + case.lower(), 'revision_id': None}
    save(folder / 'tts-request.json', request)
    environment = json.loads((ROOT / '.local/ipad-delivery/relay-environment.json').read_text())
    bind = environment.get('RUFREN_RELAY_BIND', '127.0.0.1:8787')
    if not bind.startswith('127.0.0.1:'): raise ValueError('Requires existing loopback relay')
    query = urllib.request.Request('http://' + bind + '/api/speech/generate',
        data=json.dumps(request, ensure_ascii=False).encode(), headers={'Content-Type': 'application/json',
        'Authorization': 'Bearer ' + environment['RUFREN_SYNC_TOKEN']})
    started = time.monotonic()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(query, timeout=60) as response:
            raw = response.read(4 * 1024**2 + 1)
            if len(raw) > 4 * 1024**2: raise ValueError('Oversized response')
    except Exception as error:
        save(folder / 'failure.json', {'stage': 'tts', 'error_type': type(error).__name__, 'elapsed_seconds': time.monotonic() - started})
        raise
    elapsed = time.monotonic() - started
    (folder / 'tts-response.json').write_bytes(raw)
    result = json.loads(raw)
    wav = base64.b64decode(result['wav_base64'], validate=True)
    digest = hashlib.sha256(wav).hexdigest()
    with wave.open(io.BytesIO(wav)) as audio:
        shape = {'frames': audio.getnframes(), 'sample_rate': audio.getframerate(), 'channels': audio.getnchannels()}
        if audio.getsampwidth() != 2: raise ValueError('Not PCM16')
    for field in ('request_id', 'revision_id'):
        if result[field] != request[field]: raise ValueError('Response binding mismatch')
    if result['source_text'] != request['text'] or result['sha256'] != digest: raise ValueError('Source or hash mismatch')
    if any(result[k] != v for k,v in shape.items()): raise ValueError('Shape mismatch')
    expected_provider = 'local_qwen3_tts' if case == 'RQ10' else 'local_silero_tts'
    if result['provenance']['provider'] != expected_provider: raise ValueError('Wrong provider')
    synthesis = result['provenance']['synthesis_input']
    if synthesis['synthesis_text'] != request['text'] or synthesis['source_text_sha256'] != hashlib.sha256(request['text'].encode()).hexdigest():
        raise ValueError('Wrong synthesis input')
    (folder / 'reference.wav').write_bytes(wav)
    (PUBLIC / (case + '.wav')).write_bytes(wav)
    sidecar = {k:v for k,v in result.items() if k != 'wav_base64'}
    sidecar.update(case=case, gateway_elapsed_seconds=elapsed, raw_artifacts=str(folder.relative_to(ROOT)),
                   contract='pass', human_listening=None, acoustic_assessment=None)
    save(PUBLIC / (case + '.json'), sidecar)
    print(json.dumps({'case':case, 'stage':'tts_saved', 'path':str(PUBLIC / (case+'.wav')), 'sha256':digest, 'seconds':elapsed}), flush=True)
    asr_request = {'request_id': 'reference-quality-asr-' + case.lower(), 'asset_id': 'reference-quality-slot1-' + case.lower(),
        'language':'ru', 'audio_sha256':digest, 'wav_base64':result['wav_base64']}
    save(folder / 'asr-request.json', asr_request)
    started = time.monotonic()
    with (folder / 'asr-request.json').open('rb') as source, (folder / 'asr-response.json').open('xb') as output, (folder / 'asr-stderr.log').open('xb') as errors:
        process = subprocess.run([str(ROOT / '.local/qwen3-asr/venv/bin/python'), '-B', str(ROOT / 'scripts/speech/asr_worker.py')],
           cwd=ROOT, stdin=source, stdout=output, stderr=errors, timeout=130,
           env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
    asr_elapsed = time.monotonic() - started
    if process.returncode: raise RuntimeError('Independent ASR failed; inspect saved stderr')
    evidence = json.loads((folder / 'asr-response.json').read_text())
    if evidence['audio_sha256'] != digest or evidence['target_conditioned'] is not False: raise ValueError('ASR binding mismatch')
    differences = token_differences(request['text'], evidence['transcript'])
    report = {'case':case, 'source_text':request['text'], 'sha256':digest, 'contract':'pass',
        'lexical_content':'no_lexical_mismatch' if not differences else 'suspect', 'transcript':evidence['transcript'],
        'lexical_differences':differences, 'target_contrast':'pending_offline_analysis' if case != 'RQ10' else 'not_applicable',
        'human_listening':None, 'tts_elapsed_seconds':elapsed, 'asr_elapsed_seconds':asr_elapsed,
        'asr_evidence':evidence, 'reused_verified':verification}
    save(PUBLIC / (case + '-review.json'), report)
    print(json.dumps({k:v for k,v in report.items() if k not in ('asr_evidence','reused_verified')}, ensure_ascii=False), flush=True)

if __name__ == '__main__': main()
