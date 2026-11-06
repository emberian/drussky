#!/usr/bin/env python3
"""Acquire the one authorized pinned OmniVoice BF16 snapshot; no inference."""
import hashlib
import json
import os
from pathlib import Path
import sys
import urllib.request
sys.dont_write_bytecode=True

ROOT=Path(__file__).resolve().parents[2]
STATE=ROOT/'.local/omnivoice-audition'
MODEL='mlx-community/OmniVoice-bf16'
REVISION='8fb0b754cad788aaefec690cd55c207e8a628f85'

def main():
    os.nice(10)
    STATE.mkdir(parents=True,exist_ok=True,mode=0o700)
    os.environ.update(HF_HOME=str(STATE/'hf-cache'),HF_HUB_DISABLE_TELEMETRY='1',HF_HUB_DISABLE_PROGRESS_BARS='1',
        HF_XET_HIGH_PERFORMANCE='0',PYTHONDONTWRITEBYTECODE='1')
    from importlib.metadata import distributions,version
    from huggingface_hub import snapshot_download
    if version('mlx-audio')!='0.6.4' or version('mlx')!='0.32.4': raise RuntimeError('Unexpected shared read-only runtime')
    with urllib.request.urlopen(f'https://huggingface.co/api/models/{MODEL}/tree/{REVISION}?recursive=true',timeout=30) as r:
        metadata=json.load(r)
    files={x['path']:x for x in metadata if x['type']=='file'}
    size=sum(x['size'] for x in files.values())
    if not files or size>2_000_000_000: raise ValueError('Outside bounded model inventory')
    for path in files:
        if path.startswith('/') or '..' in path.split('/'): raise ValueError('Invalid inventory')
    print(f'Acquiring {MODEL}@{REVISION}: {size:,} bytes; no inference',flush=True)
    destination=STATE/'model'
    snapshot_download(MODEL,revision=REVISION,local_dir=destination,cache_dir=STATE/'hf-cache',max_workers=2)
    verified=[]
    for name,info in sorted(files.items()):
        path=destination/name
        if path.is_symlink() or not path.is_file() or path.stat().st_size!=info['size']: raise ValueError('Wrong file inventory')
        with path.open('rb') as r: sha=hashlib.file_digest(r,'sha256').hexdigest()
        if info.get('lfs',{}).get('oid') not in (None,sha): raise ValueError('LFS digest mismatch')
        verified.append({'path':name,'bytes':info['size'],'sha256':sha})
    licenses={}
    upstream='c5fdb5ccb189668d56333f77ba2629f4cd7535f4'
    for name in ('README.md','audio_tokenizer/LICENSE','audio_tokenizer/README.md'):
        with urllib.request.urlopen(f'https://huggingface.co/k2-fsa/OmniVoice/resolve/{upstream}/{name}',timeout=30) as r:data=r.read(100000)
        path=STATE/'upstream-notices'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
        licenses[name]={'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data)}
    manifest={'schema':'rufren-omnivoice-audition-model-v1','model':MODEL,'revision':REVISION,'files':verified,
        'total_bytes':size,'upstream':'k2-fsa/OmniVoice','upstream_observed_revision':upstream,
        'conversion_upstream_revision_verified':False,'license_scope':'personal noncommercial research; upstream model card CC-BY-NC; retained codec notices',
        'upstream_notices':licenses,'runtime_interpreter':sys.executable,'runtime_usage':'existing locked ASR environment read-only; no pip mutation',
        'runtime_versions':{d.metadata['Name']:d.version for d in distributions()},
        'setup_script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    (STATE/'model-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(f'Verified {len(verified)} model files and upstream notices',flush=True)

if __name__=='__main__':main()
