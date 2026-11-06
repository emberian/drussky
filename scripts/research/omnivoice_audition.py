#!/usr/bin/env python3
"""Three authorized first takes, followed by three independent ASR screens.

New experiment only: no service/config or scripts/speech mutations. Shared
installed Python packages are used read-only; cache/model/output are separate.
"""
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

ROOT=Path(__file__).resolve().parents[2]
STATE=ROOT/'.local/omnivoice-audition'
OUT=ROOT/'research/artifacts/omnivoice-audition-v1'
CASES={
 'OVA01':'Он дома, она у окна, оно открыто, а они пьют молоко.',
 'OVA02':'Это лук, а это люк.',
 'OVA03':'У меня сегодня 2 вопроса, 5 идей и 21 свободная минута.',
}
SETTINGS={'language':'ru','instruct':'female','num_steps':32,'guidance_scale':2.0,
 'class_temperature':0.0,'position_temperature':5.0,'layer_penalty_factor':5.0,'t_shift':0.1}
for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS','MKL_NUM_THREADS','NUMEXPR_NUM_THREADS'):
    os.environ[key]='2'
os.environ.update(HF_HOME=str(STATE/'hf-cache'),HF_HUB_OFFLINE='1',TRANSFORMERS_OFFLINE='1',HF_HUB_DISABLE_TELEMETRY='1',
    TOKENIZERS_PARALLELISM='false',RAYON_NUM_THREADS='2',PYTHONDONTWRITEBYTECODE='1')

def sha(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path,value):
    with path.open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2);stream.write('\n')

def child():
    import resource
    import mlx.core as mx
    import numpy as np
    from importlib.metadata import version
    from mlx_audio.tts.utils import load_model
    mx.set_memory_limit(12*1024**3);mx.set_cache_limit(1024**3)
    if version('mlx-audio')!='0.6.4' or version('mlx')!='0.32.4':raise ValueError('Unexpected runtime')
    started=time.monotonic()
    model=load_model(str(STATE/'model'))
    # These are required; the library can otherwise synthesize placeholder silence.
    if model.audio_tokenizer is None or model.text_tokenizer is None:raise ValueError('Missing required tokenizer')
    load_seconds=time.monotonic()-started
    package=Path(sys.prefix)/'lib/python3.12/site-packages/mlx_audio'
    code=[*sorted((package/'tts/models/omnivoice').glob('*.py')),
          *sorted((package/'codec/models/higgs_audio').glob('*.py')),package/'tts/utils.py',package/'tts/models/base.py']
    runtime_hashes={str(p.relative_to(package)):sha(p) for p in code}
    for index,(case,text) in enumerate(CASES.items()):
        folder=STATE/'runs'/case;folder.mkdir()
        request={'case':case,'source_text':text,'settings':SETTINGS,'seed':42,'reference_audio':None,
            'duration_override':None,'duration_estimation':'installed MLX RuleDurationEstimator default; not aligned to baseline'}
        save(folder/'request.json',request)
        save(folder/'attempt.json',{'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tts_attempt_ceiling':1})
        mx.random.seed(42)
        started=time.monotonic();results=list(model.generate(text=text,**SETTINGS))
        if len(results)!=1:raise ValueError('Expected one whole-wave result')
        result=results[0];mx.eval(result.audio)
        x=np.asarray(result.audio,dtype=np.float32).reshape(-1)
        elapsed=time.monotonic()-started
        if result.sample_rate!=24000 or not 1<=len(x)<=24000*45 or not np.isfinite(x).all() or not np.any(x):
            raise ValueError('Invalid emitted audio')
        np.save(folder/'emitted-f32.npy',x,allow_pickle=False)
        clipped=int(np.sum((x<-1)|(x>1)))
        # Explicit stored conversion; original floating samples are also retained.
        pcm=np.rint(np.clip(x,-1,32767/32768)*32768).astype('<i2').tobytes()
        wav_path=OUT/(case+'.wav')
        with wave.open(str(wav_path),'wb') as w:
            w.setparams((1,2,24000,len(x),'NONE','not compressed'));w.writeframes(pcm)
        result_data={'schema':'rufren-experimental-tts-v1','case':case,'source_text':text,'source_text_sha256':hashlib.sha256(text.encode()).hexdigest(),
            'synthesis_text':text,'synthesis_text_sha256':hashlib.sha256(text.encode()).hexdigest(),'transform':'verbatim',
            'wav_sha256':sha(wav_path),'frames':len(x),'sample_rate':24000,'channels':1,'duration_seconds':len(x)/24000,
            'provenance':{'provider':'experimental_omnivoice_mlx','model':'mlx-community/OmniVoice-bf16',
              'revision':'8fb0b754cad788aaefec690cd55c207e8a628f85','model_manifest_sha256':sha(STATE/'model-manifest.json'),
              'settings':SETTINGS,'seed':42,'voice_mode':'attribute voice design without reference; training language generalization unvalidated',
              'runtime':{'mlx-audio':version('mlx-audio'),'mlx':version('mlx'),'numpy':version('numpy')},
              'runtime_source_sha256':runtime_hashes,'audition_script_sha256':sha(Path(__file__))},
            'timing':{'model_load_seconds':load_seconds if index==0 else None,'model_already_loaded':index>0,
              'generation_to_complete_wave_seconds':elapsed,'first_playable_audio_seconds':elapsed,
              'streaming':False,'compute_seconds_per_audio_second':elapsed/(len(x)/24000),
              'process_lifetime_peak_rss_bytes':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'metal_lifetime_peak_bytes':mx.get_peak_memory()},
            'signal':{'emitted_float_peak':float(np.max(np.abs(x))),'emitted_out_of_range_samples':clipped,
              'pcm_conversion':'round(clip(x,-1,32767/32768)*32768) int16le','emitted_float_sha256':sha(folder/'emitted-f32.npy')},
            'library_stats':{'token_count':result.token_count,'reported_processing_time_seconds':result.processing_time_seconds,
              'reported_real_time_factor':result.real_time_factor,'note':'Library RTF is audio/compute; use explicit compute/audio metric above'},
            'human_listening':None,'acoustic_correctness':None}
        save(OUT/(case+'.json'),result_data)
        save(folder/'response.json',{**result_data,'wav_base64':base64.b64encode(wav_path.read_bytes()).decode()})
        print(json.dumps({'case':case,'sha256':result_data['wav_sha256'],'audio_seconds':len(x)/24000,
            'generation_seconds':elapsed,'rss':result_data['timing']['process_lifetime_peak_rss_bytes'],
            'metal_peak':result_data['timing']['metal_lifetime_peak_bytes']},ensure_ascii=False),flush=True)

def main():
    if len(sys.argv)>1 and sys.argv[1]=='--child':child();return
    OUT.mkdir(parents=True,exist_ok=True)
    (STATE/'runs').mkdir()  # Exclusive experiment; no automatic repeat after failure.
    manifest=json.loads((STATE/'model-manifest.json').read_text())
    for item in manifest['files']:
        if sha(STATE/'model'/item['path'])!=item['sha256']:raise ValueError('Pinned model asset changed')
    save(STATE/'runs'/'authorization.json',{'tts_ceiling':3,'asr_ceiling':3,'retries':0,'source_texts':CASES,
        'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'setup_manifest_sha256':sha(STATE/'model-manifest.json')})
    with (STATE/'runs'/'tts-stderr.log').open('xb') as errors:
        process=subprocess.Popen([sys.executable,'-B',str(Path(__file__)),'--child'],cwd=ROOT,stderr=errors,start_new_session=True)
        started=time.monotonic()
        try:
            while process.poll() is None:
                if time.monotonic()-started>300:raise TimeoutError('Three-item inference deadline')
                r=subprocess.run(['/bin/ps','-o','rss=','-p',str(process.pid)],capture_output=True,text=True,timeout=3)
                if r.stdout.strip() and int(r.stdout.strip())*1024>16*1024**3:raise MemoryError('Owned child RSS limit')
                time.sleep(.25)
        except BaseException:
            os.killpg(process.pid,signal.SIGKILL);process.wait();raise
        if process.returncode:raise RuntimeError('TTS failed; saved first-take artifacts/stderr, no retry')
    # TTS process/model is gone before independent recognizer loading.
    for case,text in CASES.items():
        folder=STATE/'runs'/case;s=json.loads((OUT/(case+'.json')).read_text());wav=(OUT/(case+'.wav')).read_bytes()
        request={'request_id':'omnivoice-v1-asr-'+case.lower(),'asset_id':'omnivoice-v1-'+case.lower(),
            'audio_sha256':s['wav_sha256'],'language':'ru','wav_base64':base64.b64encode(wav).decode()}
        save(folder/'asr-request.json',request)
        started=time.monotonic()
        with (folder/'asr-request.json').open('rb') as source,(folder/'asr-response.json').open('xb') as output,(folder/'asr-stderr.log').open('xb') as errors:
            result=subprocess.run([sys.executable,'-B',str(ROOT/'scripts/speech/asr_worker.py')],cwd=ROOT,stdin=source,stdout=output,stderr=errors,timeout=130)
        if result.returncode:raise RuntimeError('ASR failed, no retry')
        evidence=json.loads((folder/'asr-response.json').read_text())
        if evidence['audio_sha256']!=s['wav_sha256'] or evidence['target_conditioned'] is not False:raise ValueError('ASR binding mismatch')
        review={'case':case,'source_text':text,'wav_sha256':s['wav_sha256'],'asr_elapsed_seconds':time.monotonic()-started,
            'literal_transcript':evidence['transcript'],'asr_evidence':evidence,'human_listening':None,'acoustic_correctness':None}
        save(OUT/(case+'-review.json'),review)
        print(json.dumps({k:v for k,v in review.items() if k!='asr_evidence'},ensure_ascii=False),flush=True)

if __name__=='__main__':main()
