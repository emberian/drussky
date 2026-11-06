#!/usr/bin/env python3
"""One-shot preparation of the agreed 11 foundation models; never a learner runtime path."""
import subprocess,json,base64,pathlib,hashlib,wave,io,array,datetime
root=pathlib.Path(__file__).resolve().parents[1]; out=root/'crates/rufren-app/assets/foundations'
if any(out.glob('*.wav')):
 raise SystemExit('Refusing to regenerate existing original models; review and authorize a new revision first.')
out.mkdir(parents=True, exist_ok=True)
items=[('a','а'),('ma','ма'),('am','ам'),('da','да'),('net','нет'),('o','о'),('ka','ка'),('ta','та'),('kot','кот'),('tam','там'),('tam-kot','Там кот.')]
manifest={'format':'rufren-foundation-audio-v1','created_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'prepared synthetic reference; lexical/phonemic naturalness not independently certified','assets':[]}
for short,text in items:
 id='foundation-'+short
 req={'requests':[{'request_id':id,'revision_id':'foundation-v1','text':text}]}
 result=subprocess.run([str(root/'.local/silero-tts/venv/bin/python'),str(root/'scripts/speech/silero_worker.py')],input=json.dumps(req,ensure_ascii=False),text=True,capture_output=True,timeout=45)
 if result.returncode: raise RuntimeError((id,result.stderr))
 data=json.loads(result.stdout); response=data['responses'][0]; wav=base64.b64decode(response.pop('wav_base64'),validate=True)
 assert hashlib.sha256(wav).hexdigest()==response['sha256']
 with wave.open(io.BytesIO(wav)) as w:
  assert (w.getnchannels(),w.getsampwidth(),w.getframerate())==(1,2,24000)
  assert w.getnframes()==response['frames']
  pcm=array.array('h',w.readframes(w.getnframes())); assert max(abs(x) for x in pcm)>100
 assert data['measurements'][0]['clipped_samples']==0
 assert .1 <= response['frames']/24000 <= 6
 (out/(id+'.wav')).write_bytes(wav)
 manifest['assets'].append({'id':id,'file':id+'.wav','spoken':text,'display_text':text,'teaching_span':{'ma':'initial м in ма; whole syllable also used','ka':'initial к in ка','ta':'initial т in та','da':'whole yes chunk; initial д introduced in episode 3'}.get(short,'whole spoken target'),'encoding':'RIFF/WAVE PCM16 little-endian mono','byte_length':len(wav),'source_response':response,'measurements':data['measurements'][0],'runtime':{k:data[k] for k in ['runtime','device','threads','model_license','model_source_revision']},'transforms':[]})
 (out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
 print(id,round(response['frames']/24000,3),response['sha256'],flush=True)
