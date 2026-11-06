#!/usr/bin/env python3
"""Deterministic geometric block-print fixtures, not human handwriting."""
from pathlib import Path
import argparse,math,json,hashlib
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out',type=Path,required=True,help='A new output directory for blinded strokes and separate labels')
out=parser.parse_args().out
out.mkdir(parents=True,exist_ok=False)
# Coordinates in one 20x28 point uppercase block-print box.
glyphs={
 'К': [[(2,2),(2,26)],[(17,2),(2,14),(18,26)]],
 'О': [[(10+8*math.cos(-math.pi/2+i*2*math.pi/48),14+12*math.sin(-math.pi/2+i*2*math.pi/48)) for i in range(49)]],
 'Т': [[(1,2),(19,2)],[(10,2),(10,26)]],
 'А': [[(1,26),(10,2),(19,26)],[(5,17),(15,17)]],
 'М': [[(1,26),(1,2),(10,20),(19,2),(19,26)]],
 'Н': [[(2,2),(2,26)],[(2,14),(18,14)],[(18,2),(18,26)]],
 '.': [[(8,25),(8.1,25.1)]],
}
labels=[('ink001','КОТ'),('ink002','ОНА'),('ink003','ТАМ КОТ.')]
fixtures=[]
for fid,text in labels:
 strokes=[];cursor=24;t=0
 for c in text:
  if c==' ':cursor+=12;continue
  for path in glyphs[c]:
   points=[]
   for j,(a,b) in enumerate(zip(path,path[1:])):
    count=max(1,math.ceil(math.dist(a,b)/1.5))
    for k in range(count):
     frac=k/count;points.append({'x':cursor+a[0]+frac*(b[0]-a[0]),'y':24+a[1]+frac*(b[1]-a[1]),'t':t});t+=12
   points.append({'x':cursor+path[-1][0],'y':24+path[-1][1],'t':t});t+=80
   strokes.append({'points':points})
  cursor+=24
 fixtures.append({'id':fid,'width':300,'height':90,'strokes':strokes})
payload={'schema':'synthetic-print-ink-v1','locale':'ru','units':'points on ordinary paper','provenance':'Geometric block-print fixtures; not human or Apple Pencil input','fixtures':fixtures}
path=out/'synthetic-ink.json';path.write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n')
(out/'synthetic-ink-labels.json').write_text(json.dumps(dict(labels),ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'fixtures':len(fixtures),'input_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'expected_answers_in_input':False}))
