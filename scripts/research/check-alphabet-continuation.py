#!/usr/bin/env python3
"""Validate the unshipped alphabet continuation against the current authored course.

This checks structure and exact bindings, not pronunciation quality or learning.
"""
import json,pathlib,re,hashlib
r=pathlib.Path(__file__).resolve().parents[2];d=r/'crates/rufren-content/content/extensions'
base=json.load(open(r/'crates/rufren-content/content/foundations-v1.json'));p=json.load(open(d/'alphabet-construction-v1.json'));b=json.load(open(d/'alphabet-construction-bindings-v1.json'));w=json.load(open(d/'alphabet-construction-handwriting-v1.json'));g=json.load(open(d/'alphabet-construction-glyphs-v1.json'))
shapes=lambda s:{c.upper() for c in s if 'А'<=c<='я' or c in 'Ёё'}
known=set();mean=set();samples=set();conv=set();ids=set();sources={}; assets={a['id']:a['spoken'] for a in base['assets']+p['assets']}; previous=set(); errors=[]
for e in base['episodes']+p['episodes']:
 assert e['id'] not in ids;ids.add(e['id'])
 for s in e['steps']:
  assert s['id'] not in ids;ids.add(s['id'])
  for required,k,label in [('requires_graphemes',known,'shape'),('requires_meanings',mean,'meaning'),('requires_samples',samples,'sample'),('requires_conventions',conv,'convention')]:
   missing=set(s.get(required,[]))-k
   if missing:errors.append((s['id'],label,sorted(missing)))
  known.update(s['introduces_graphemes']);mean.update(s['introduces_meanings']);samples.update(s.get('introduces_samples',[]));conv.update(s.get('introduces_conventions',[]));previous.add(s['id'])
  assert all(a in assets for a in s['audio_ids'])
  a=s['action'];kind=a['kind']; visible=s['instruction']+' '+(s['display'] or '')
  if kind=='present':visible+=' '+a['note']
  elif kind=='complete':visible+=' '+a['summary']
  elif kind=='blend':
   assert len(a['answer'])==len(a['feedback']) and all(x in a['tiles'] for x in a['answer'])
   visible+=' '+' '.join(a['tiles']+a['feedback'])+' '+a['success']
  elif kind=='recall':
   assert a['answer'] and a['reveal'] and s['audio_ids'];mode=a['mode']
   assert (mode=='reading_meaning')==(s['display'] is not None)
   assert (mode=='written')==bool(a['keyboard'])
   assert (mode=='spoken')==(not a['accepted'])
   if mode=='written':assert shapes(a['answer'])<=shapes(''.join(a['keyboard']))
   visible+=' '+' '.join(a['keyboard'])+' '+a['reveal']+' '+' '.join(f['feedback'] for f in a['feedback'])
   if not s['audio_first_chunk']:visible+=' '+a['answer']
  elif kind=='choose':visible+=' '+a['success']+' '+' '.join(o['text']+' '+o['feedback'] for o in a['options'])
  elif kind in ['read','copy','speak']:visible+=' '+a['note']+' '+a.get('reveal_text','')+' '+a.get('answer','')+' '+''.join(a.get('keyboard',[]))
  if shapes(visible)-known:errors.append((s['id'],'visible unknown',sorted(shapes(visible)-known)))
  if '\u0301' in visible and 'stress-acute-annotation' not in conv:errors.append((s['id'],'unintroduced stress annotation'))
  sources[s['id']]=(s,set(known),set(mean),set(samples),set(conv),set(previous))
allglyphs=set(g['existing_models_reused'])|{x['glyph'] for x in g['glyphs']}|{x['glyph'] for x in g['punctuation_additions']}
for t in w:
 s,k,m,sa,c,prev=sources[t['foundation_step_id']]
 assert s['action']['kind']=='present' and s['revision']==t['foundation_step_revision']
 assert set(t['requires_graphemes'])<=k and set(t['requires_meanings'])<=m and set(t['requires_samples'])<=sa and set(t['requires_conventions'])<=c
 assert set(t['prerequisite_step_ids'])<=prev and t['foundation_step_id'] in t['prerequisite_step_ids']
 assert shapes(t['target'])<=set(t['requires_graphemes']) and shapes(t['target'])<=set(t['required_models'])
 assert all(x in allglyphs for x in t['target']),(t['id'],t['target'])
 assert t['audio_id'] in s['audio_ids']
 spoken=assets[t['audio_id']].lower();target=t['target'].lower()
 assert spoken==target if t['input_focus']=='whole_target' else spoken.startswith(target) and len(target)==1 and len(spoken)>1
assert known==set('АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ') and set(b['letter_introduction_steps'])==set('ЯЫЕЭСЛРУВБПЗГХЖШЁЮЙЬЦЧЩФЪ')
assert len(set(a['id'] for a in b['audio_requests']))==len(p['assets'])
assert {a['id'] for a in b['audio_requests']}=={a['id'] for a in p['assets']}
for request in b['audio_requests']:
 assert request['ordinary_spelling']==assets[request['id']]
 assert request['marked_stress_request'].replace('\u0301','')==request['ordinary_spelling']
assert {s['id'] for e in p['episodes'] for s in e['steps'] if s['action']['kind']=='recall'}=={x['step_id'] for x in b['recall_bindings']}
assert not errors,errors
print(json.dumps({'result':'passed structural editorial audit only','continuation':b['inventory'],'merged_letters':len(known),'glyph_models':len(g['glyphs']),'base_pack_sha256':hashlib.sha256((r/'crates/rufren-content/content/foundations-v1.json').read_bytes()).hexdigest(),'checks':['prior prerequisite declarations','visible and response Cyrillic coverage','all references and audio request IDs','ordinary and marked-stress request text identity','current action invariants','whole-target and initial-segment audio binding','handwriting case/punctuation glyph coverage','complete 33-letter inventory','exact recall binding coverage']},ensure_ascii=False,indent=2))
