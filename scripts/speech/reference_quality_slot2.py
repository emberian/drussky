#!/usr/bin/env python3
"""Acquire only RQ06–09 (one TTS each) and RQ01/06–09 (one ASR each).

Uses existing main services and pinned ASR. Attempt directories are exclusive:
failure never permits an automatic retry. No expected text reaches recognition.
"""
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
import urllib.error
import urllib.request
import wave

sys.dont_write_bytecode = True
from asr_review import token_differences
from silero_frontend import synthesis_input
from reference_quality_slot1 import NoRedirect, save

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / '.local/reference-quality/slot2'
PUBLIC = ROOT / 'research/artifacts/reference-quality-slot2'
CASES = {
    'RQ01': 'Здравствуйте!',
    'RQ06': 'На двери висит замок, а на холме стоит старый замок.',
    'RQ07': 'У меня сегодня 2 вопроса, 5 идей и 21 свободная минута.',
    'RQ08': 'У меня сегодня два вопроса, пять идей и двадцать одна свободная минута.',
    'RQ09': 'У меня́ сего́дня два вопро́са, пять иде́й и два́дцать одна́ свобо́дная мину́та.',
}
GREETING = ROOT / '.local/silero-tts/samples/hybrid-gateway-greeting-v1.wav'
GREETING_SHA = '1f431043b464123eb9a023fe2cd52ecaccdca6f15f8a42d82b4328202ed2b138'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def frozen_slot1():
    folder = ROOT / 'research/artifacts/reference-quality-slot1'
    manifest = json.loads((folder / 'evaluation.json').read_text())
    for name, expected in manifest['artifact_hashes'].items():
        if digest((folder / name).read_bytes()) != expected:
            raise ValueError('Frozen slot1 artifact changed')
    return digest((folder / 'evaluation.json').read_bytes())

def main():
    case = sys.argv[1]
    if case not in CASES: raise ValueError('Outside authorized slot')
    OUT.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    frozen = frozen_slot1()
    if digest(GREETING.read_bytes()) != GREETING_SHA: raise ValueError('Reused greeting changed')
    folder = OUT / case
    folder.mkdir()  # Prevent duplicate model calls, including after a failed attempt.
    save(folder / 'attempt.json', {'case':case, 'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'script_sha256':digest(Path(__file__).read_bytes()),'slot1_evaluation_sha256':frozen,
        'dependency_sha256':{name:digest((ROOT/'scripts/speech'/name).read_bytes()) for name in ('reference_quality_slot1.py','asr_review.py','silero_frontend.py')},
        'tts_attempt_ceiling':0 if case=='RQ01' else 1,'asr_attempt_ceiling':1})
    elapsed = None
    if case == 'RQ01':
        wav = GREETING.read_bytes()
        result = json.loads(GREETING.with_suffix('.json').read_text())
        result['wav_base64'] = base64.b64encode(wav).decode()
        save(folder / 'reuse.json', {'path':str(GREETING.relative_to(ROOT)),'sha256':GREETING_SHA,'new_tts_calls':0})
    else:
        request = {'text':CASES[case],'request_id':'reference-quality-slot2-'+case.lower(),'revision_id':None}
        save(folder / 'tts-request.json', request)
        config = json.loads((ROOT/'.local/ipad-delivery/relay-environment.json').read_text())
        bind = config.get('RUFREN_RELAY_BIND','127.0.0.1:8787')
        if not bind.startswith('127.0.0.1:'): raise ValueError('Requires existing loopback relay')
        query = urllib.request.Request('http://'+bind+'/api/speech/generate',
            data=json.dumps(request,ensure_ascii=False).encode(),headers={'Content-Type':'application/json',
            'Authorization':'Bearer '+config['RUFREN_SYNC_TOKEN']})
        started = time.monotonic()
        try:
            with urllib.request.build_opener(urllib.request.ProxyHandler({}),NoRedirect()).open(query,timeout=60) as response:
                raw = response.read(4*1024**2+1)
                (folder/'tts-response.json').write_bytes(raw)
                if len(raw)>4*1024**2: raise ValueError('Response exceeds bound')
        except Exception as error:
            if isinstance(error,urllib.error.HTTPError): (folder/'tts-error-response.txt').write_bytes(error.read(65536))
            save(folder/'failure.json',{'stage':'tts','error_type':type(error).__name__,'elapsed_seconds':time.monotonic()-started})
            raise
        elapsed = time.monotonic()-started
        result = json.loads(raw)
        wav = base64.b64decode(result['wav_base64'],validate=True)
        for key in ('request_id','revision_id'):
            if result[key] != request[key]: raise ValueError('Request binding mismatch')
    sha = digest(wav)
    if sha != result['sha256'] or result['source_text'] != CASES[case]: raise ValueError('Source/hash mismatch')
    with wave.open(io.BytesIO(wav)) as w:
        if w.getsampwidth()!=2: raise ValueError('Not PCM16')
        shape={'sample_rate':w.getframerate(),'channels':w.getnchannels(),'frames':w.getnframes()}
    if any(result[key]!=value for key,value in shape.items()): raise ValueError('Audio shape mismatch')
    expected_provider='local_silero_tts' if case in ('RQ01','RQ09') else 'local_qwen3_tts'
    if result['provenance']['provider'] != expected_provider: raise ValueError('Unexpected leaf provider')
    expected_input=synthesis_input(CASES[case])
    if result['provenance']['synthesis_input'] != expected_input: raise ValueError('Unexpected synthesis input')
    if case == 'RQ09':
        for flag in ('put_accent','put_stress_homo','put_yo','put_yo_homo','stress_single_vowel'):
            if flag+'=false' not in result['provenance']['validation']: raise ValueError('Explicit stress flag mismatch')
    with (folder/'reference.wav').open('xb') as stream: stream.write(wav)
    with (PUBLIC/(case+'.wav')).open('xb') as stream: stream.write(wav)
    sidecar={key:value for key,value in result.items() if key!='wav_base64'}
    sidecar.update(case=case,slot2_gateway_elapsed_seconds=elapsed,raw_artifacts=str(folder.relative_to(ROOT)),
        slot2_tts_calls=0 if case=='RQ01' else 1,contract='pass',human_listening=None)
    save(PUBLIC/(case+'.json'),sidecar)
    print(json.dumps({'case':case,'stage':'source_saved','sha256':sha,'seconds':elapsed},ensure_ascii=False),flush=True)
    asr={'request_id':'reference-quality-slot2-asr-'+case.lower(),'asset_id':'reference-quality-slot2-'+case.lower(),
        'language':'ru','audio_sha256':sha,'wav_base64':result['wav_base64']}
    save(folder/'asr-request.json',asr)
    started=time.monotonic()
    with (folder/'asr-request.json').open('rb') as source, (folder/'asr-response.json').open('xb') as output, (folder/'asr-stderr.log').open('xb') as errors:
        process=subprocess.run([str(ROOT/'.local/qwen3-asr/venv/bin/python'),'-B',str(ROOT/'scripts/speech/asr_worker.py')],
            cwd=ROOT,stdin=source,stdout=output,stderr=errors,timeout=130,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
    asr_elapsed=time.monotonic()-started
    if process.returncode: raise RuntimeError('ASR failed; raw result retained, no retry')
    evidence=json.loads((folder/'asr-response.json').read_text())
    if evidence['audio_sha256']!=sha or evidence['target_conditioned'] is not False: raise ValueError('ASR binding mismatch')
    expected_spoken=CASES['RQ08'] if case in ('RQ07','RQ09') else CASES[case]
    differences=token_differences(expected_spoken,evidence['transcript'])
    review={'case':case,'source_text':CASES[case],'expected_spoken_words':expected_spoken,'sha256':sha,'contract':'pass',
        'transcript':evidence['transcript'],'lexical_differences_from_expected_words':differences,
        'lexical_status':'no_lexical_mismatch' if not differences else 'needs_postrecognition_review',
        'note':'Literal transcript retained. Numeric glyphs can conceal spoken gender/inflection; never infer them from value alone.',
        'target_contrast':'unresolved_contextual_stress' if case=='RQ06' else 'not_assessed_by_transcript',
        'human_listening':None,'tts_elapsed_seconds':elapsed,'asr_elapsed_seconds':asr_elapsed,'asr_evidence':evidence}
    save(PUBLIC/(case+'-review.json'),review)
    if frozen_slot1()!=frozen: raise ValueError('Slot1 changed during acquisition')
    print(json.dumps({key:value for key,value in review.items() if key!='asr_evidence'},ensure_ascii=False),flush=True)

if __name__=='__main__': main()
