#!/usr/bin/env python3
"""Independent offline lexical screen. Inputs contain only IDs, paths and hashes.

Single pinned model, sequential whole recordings, no expected text/hotwords or
audio edits. Run under run-bounded.py; writes transcripts, never grades phones.
"""
import argparse
import contextlib
from datetime import datetime, timezone
import fcntl
import hashlib
from importlib.metadata import version
import json
import math
import os
from pathlib import Path
import signal
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts/speech'))
sys.dont_write_bytecode = True
from asr_worker import decode_wav
from asr_model import verify_install, STATE, MODEL_PATH, MODEL_ID, MODEL_REVISION, RUNTIME_VERSION, MANIFEST


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('inputs', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    inputs = json.loads(args.inputs.read_text())
    assert 0 < len(inputs) <= 112 and len({r['id'] for r in inputs}) == len(inputs)
    assert all(set(r) == {'id', 'path', 'sha256'} for r in inputs), 'Do not supply target text to recognition'
    args.output.mkdir(parents=True, exist_ok=False)
    if version('mlx-audio') != RUNTIME_VERSION:
        raise ValueError('Unexpected pinned ASR runtime')
    with (STATE / 'inference.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        verify_install()
        import numpy as np
        import mlx.core as mx
        from scipy.signal import resample_poly
        from mlx_audio.stt import load
        mx.set_memory_limit(8 * 1024**3)
        mx.set_cache_limit(256 * 1024**2)
        os.nice(10)
        def expired(_signal, _frame):
            raise TimeoutError('One fixed transcription exceeded 60 seconds')
        signal.signal(signal.SIGALRM, expired)
        with (args.output / 'runtime.log').open('x') as log:
            with contextlib.redirect_stdout(log), contextlib.redirect_stderr(log):
                model = load(str(MODEL_PATH))
            rows = []
            for item in inputs:
                data = Path(item['path']).read_bytes()
                assert sha(data) == item['sha256']
                shape, payload = decode_wav(data)
                values = np.frombuffer(payload, dtype='<i2' if shape['bits'] == 16 else '<f4').astype(np.float32)
                if shape['bits'] == 16:
                    values /= 32768.0
                source_pcm_hash = sha(values.astype('<f4').tobytes())
                values = values.reshape(-1, shape['channels']).mean(axis=1)
                common = math.gcd(shape['sample_rate'], 16000)
                if shape['sample_rate'] != 16000:
                    values = resample_poly(values, 16000 // common, shape['sample_rate'] // common).astype(np.float32)
                mx.random.seed(42)
                started = time.monotonic()
                signal.alarm(60)
                with contextlib.redirect_stdout(log), contextlib.redirect_stderr(log):
                    result = model.generate(values, language='Russian', temperature=0.0, max_tokens=128, batch_size=1, verbose=False, system_prompt=None, hotwords=None)
                signal.alarm(0)
                row = {'id': item['id'], 'audio_sha256': item['sha256'], 'original_audio': shape, 'source_pcm_sha256': source_pcm_hash, 'derivative': {'sample_rate': 16000, 'frames': len(values), 'pcm_sha256': sha(values.astype('<f4').tobytes()), 'method': 'arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5', 'scipy_version': version('scipy'), 'amplitude_normalization': False}, 'transcript': result.text, 'generation_tokens': result.generation_tokens, 'complete': result.generation_tokens < 128, 'elapsed_seconds': round(time.monotonic()-started, 4), 'target_conditioned': False, 'acoustic_assessment': None}
                (args.output / (item['id'] + '.json')).write_text(json.dumps(row, ensure_ascii=False, indent=2)+'\n')
                rows.append(row)
                print(json.dumps({'id': item['id'], 'transcript': result.text, 'complete': row['complete']},ensure_ascii=False),flush=True)
                mx.clear_cache()
        manifest = {'status': 'independent transcript hypotheses; not stress/softness/naturalness verdicts', 'model': MODEL_ID, 'revision': MODEL_REVISION, 'runtime': 'mlx-audio==' + RUNTIME_VERSION, 'mlx_version': version('mlx'), 'model_manifest_sha256': sha(MANIFEST.read_bytes()), 'script_sha256': sha(Path(__file__).read_bytes()), 'inputs_sha256': sha(args.inputs.read_bytes()), 'decoding': {'temperature': 0.0, 'max_tokens': 128, 'seed': 42, 'language': 'Russian', 'hotwords': None, 'system_prompt': None}, 'completed_at': datetime.now(timezone.utc).isoformat(), 'metal_peak_bytes': mx.get_peak_memory(), 'results': rows}
        (args.output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n')


if __name__ == '__main__':
    main()
