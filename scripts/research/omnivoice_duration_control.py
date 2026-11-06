#!/usr/bin/env python3
"""One supported explicit-duration control of frozen OVA01; no retries."""
import base64
import datetime
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import wave
sys.dont_write_bytecode=True
import omnivoice_audition as baseline_code

ROOT=baseline_code.ROOT
STATE=baseline_code.STATE
BASE=ROOT/'research/artifacts/omnivoice-audition-v1'
OUT=ROOT/'research/artifacts/omnivoice-duration-control-v1'
RUN=STATE/'duration-control-v1'
CASE='OVA01-duration45'
REQUESTED_DURATION=4.5
save=baseline_code.save
sha=baseline_code.sha

def child():
    import resource
    import mlx.core as mx
    import numpy as np
    from importlib.metadata import version
    from mlx_audio.tts.utils import load_model
    original=json.loads((BASE/'OVA01.json').read_text())
    settings=original['provenance']['settings']
    assert settings==baseline_code.SETTINGS
    assert original['provenance']['seed']==42
    package=Path(sys.prefix)/'lib/python3.12/site-packages/mlx_audio'
    for name,expected in original['provenance']['runtime_source_sha256'].items():
        if sha(package/name)!=expected:raise ValueError('Runtime differs from original')
    if version('mlx-audio')!='0.6.4' or version('mlx')!='0.32.4':raise ValueError('Runtime version differs')
    mx.set_memory_limit(12*1024**3);mx.set_cache_limit(1024**3)
    started=time.monotonic();model=load_model(str(STATE/'model'));load_seconds=time.monotonic()-started
    if model.audio_tokenizer is None or model.text_tokenizer is None:raise ValueError('Missing tokenizer')
    request={'case':CASE,'source_text':original['source_text'],'settings':settings,'seed':42,'reference_audio':None,
        'duration_s':REQUESTED_DURATION,'only_request_difference':'duration_s=None -> 4.5',
        'baseline_wav_sha256':original['wav_sha256'],'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat()}
    save(RUN/'request.json',request)
    mx.random.seed(42)
    started=time.monotonic();results=list(model.generate(text=original['source_text'],duration_s=REQUESTED_DURATION,**settings))
    if len(results)!=1:raise ValueError('Unexpected result count')
    result=results[0];mx.eval(result.audio);x=np.asarray(result.audio,dtype=np.float32).reshape(-1)
    elapsed=time.monotonic()-started
    if result.sample_rate!=24000 or len(x)!=113*960 or not np.isfinite(x).all() or not np.any(x):raise ValueError('Unexpected explicit-duration output')
    np.save(RUN/'emitted-f32.npy',x,allow_pickle=False)
    pcm=np.rint(np.clip(x,-1,32767/32768)*32768).astype('<i2').tobytes()
    path=OUT/(CASE+'.wav')
    with wave.open(str(path),'wb') as w:
        w.setparams((1,2,24000,len(x),'NONE','not compressed'));w.writeframes(pcm)
    provenance={**original['provenance'],'duration_s':REQUESTED_DURATION,'duration_tokens':113,
        'audition_script_sha256':sha(Path(__file__)),'baseline_audition_script_sha256':original['provenance']['audition_script_sha256']}
    data={'schema':'rufren-experimental-tts-v1','case':CASE,'source_text':original['source_text'],
        'source_text_sha256':original['source_text_sha256'],'synthesis_text':original['synthesis_text'],
        'synthesis_text_sha256':original['synthesis_text_sha256'],'transform':'verbatim','wav_sha256':sha(path),
        'frames':len(x),'sample_rate':24000,'channels':1,'duration_seconds':len(x)/24000,
        'provenance':provenance,'comparison':{'baseline_path':str((BASE/'OVA01.wav').relative_to(ROOT)),
            'baseline_sha256':original['wav_sha256'],'only_request_difference':'duration_s=None -> 4.5',
            'note':'Changing diffusion sequence length can alter the entire take; not a time-stretch or appended silence'},
        'timing':{'model_load_seconds':load_seconds,'generation_to_complete_wave_seconds':elapsed,
            'first_playable_audio_seconds':elapsed,'streaming':False,'compute_seconds_per_audio_second':elapsed/(len(x)/24000),
            'process_lifetime_peak_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'metal_lifetime_peak_bytes':mx.get_peak_memory()},
        'signal':{'emitted_float_peak':float(np.max(np.abs(x))),'emitted_out_of_range_samples':int(np.sum((x<-1)|(x>1))),
            'pcm_conversion':'round(clip(x,-1,32767/32768)*32768) int16le','emitted_float_sha256':sha(RUN/'emitted-f32.npy')},
        'library_stats':{'token_count':result.token_count,'reported_processing_time_seconds':result.processing_time_seconds,
            'reported_real_time_factor':result.real_time_factor},'human_listening':None,'acoustic_correctness':None}
    save(OUT/(CASE+'.json'),data)
    save(RUN/'response.json',{**data,'wav_base64':base64.b64encode(path.read_bytes()).decode()})
    print(json.dumps({'case':CASE,'sha256':data['wav_sha256'],'generation_seconds':elapsed,'audio_seconds':len(x)/24000,
        'load_seconds':load_seconds,'rss':data['timing']['process_lifetime_peak_rss_bytes'],'metal_peak':data['timing']['metal_lifetime_peak_bytes']}),flush=True)

def main():
    if len(sys.argv)>1 and sys.argv[1]=='--child':child();return
    RUN.mkdir();OUT.mkdir(parents=True)  # No inference repeats or overwrite.
    original=json.loads((BASE/'OVA01.json').read_text())
    if sha(BASE/'OVA01.wav')!=original['wav_sha256']:raise ValueError('Original changed')
    if sha(STATE/'model-manifest.json')!=original['provenance']['model_manifest_sha256']:raise ValueError('Model manifest changed')
    manifest=json.loads((STATE/'model-manifest.json').read_text())
    for item in manifest['files']:
        if sha(STATE/'model'/item['path'])!=item['sha256']:raise ValueError('Model changed')
    save(RUN/'authorization.json',{'tts_ceiling':1,'asr_ceiling':1,'retry_ceiling':0,'requested_duration_s':4.5,
        'baseline_sha256':original['wav_sha256'],'source_inspection':'generate(duration_s) sets ceil(duration_s*25) masked audio tokens before inference',
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()})
    with (RUN/'tts-stderr.log').open('xb') as errors:
        process=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),'--child'],cwd=ROOT,stderr=errors,start_new_session=True)
        started=time.monotonic()
        try:
            while process.poll() is None:
                if time.monotonic()-started>120:raise TimeoutError('Control deadline')
                p=subprocess.run(['/bin/ps','-o','rss=','-p',str(process.pid)],capture_output=True,text=True,timeout=3)
                if p.stdout.strip() and int(p.stdout.strip())*1024>16*1024**3:raise MemoryError('RSS cap')
                time.sleep(.25)
        except BaseException:
            os.killpg(process.pid,signal.SIGKILL);process.wait();raise
        if process.returncode:raise RuntimeError('Control failed; no retry')
    audio=OUT/(CASE+'.wav')
    request={'request_id':'omnivoice-duration45-asr','asset_id':'omnivoice-duration45','audio_sha256':sha(audio),
        'language':'ru','wav_base64':base64.b64encode(audio.read_bytes()).decode()}
    save(RUN/'asr-request.json',request)
    started=time.monotonic()
    with (RUN/'asr-request.json').open('rb') as source,(RUN/'asr-response.json').open('xb') as output,(RUN/'asr-stderr.log').open('xb') as errors:
        result=subprocess.run([sys.executable,'-B',str(ROOT/'scripts/speech/asr_worker.py')],cwd=ROOT,stdin=source,stdout=output,stderr=errors,timeout=130)
    if result.returncode:raise RuntimeError('ASR failed; no retry')
    evidence=json.loads((RUN/'asr-response.json').read_text())
    if evidence['audio_sha256']!=request['audio_sha256'] or evidence['target_conditioned'] is not False:raise ValueError('ASR binding')
    review={'case':CASE,'source_text':original['source_text'],'wav_sha256':sha(audio),'literal_transcript':evidence['transcript'],
        'asr_elapsed_seconds':time.monotonic()-started,'asr_evidence':evidence,'human_listening':None}
    save(OUT/(CASE+'-review.json'),review)
    print(json.dumps({k:v for k,v in review.items() if k!='asr_evidence'},ensure_ascii=False),flush=True)

if __name__=='__main__':main()
