#!/usr/bin/env python3
"""Offline descriptive review; no inference, alignment, or pronunciation scoring."""
import os
for k in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS','MKL_NUM_THREADS'): os.environ[k]='1'
import sys, json, hashlib, wave, subprocess
from pathlib import Path
import numpy as np
from scipy import signal
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts/speech'))
from audit_silero_stress import tracks
OUT=ROOT/'research/artifacts/alphabet-audio-review-v1'
TMP=ROOT/'.local/alphabet-audio-preparation/acoustic-review-v1'
H=ROOT/'.local/alphabet-audio-source-review'
F=ROOT/'crates/rufren-app/assets/foundations'
S=ROOT/'.local/alphabet-audio-preparation/silero-first-takes-v1'
GROUPS={'vowels':[(k,H/(k+'.wav')) for k in ('у','ю','ё','э')]+[(k,F/('foundation-'+v+'.wav')) for k,v in [('а','a'),('о','o'),('и','i')]],'laterals':[(k,H/(k+'.wav')) for k in ('лук','люк')]+[(k,ROOT/'research/artifacts/reference-quality-slot1'/(v+'.wav')) for k,v in [('RQ02 Лук','RQ02'),('RQ03 Люк','RQ03')]],'my':[(k,S/(v+'.wav')) for k,v in [('моё','foundation-a17-my-n'),('моё окно','foundation-a17-my-window')]]}
def main():
 OUT.mkdir(parents=True,exist_ok=True);TMP.mkdir(parents=True,exist_ok=True)
 report={'scope':'Descriptive acoustic review only. Active intervals are level gates, not phone boundaries; LPC poles are candidate resonances, not phonetic labels.','cases':{}}
 for group,cases in GROUPS.items():
  commands=["set terminal pngcairo size 1500,%d enhanced font 'Arial,11'"%(230*len(cases)),f"set output '{OUT}/{group}.png'",f"set multiplot layout {len(cases)},2 rowsfirst",'set palette defined (0 "#07101e", .35 "#254e70", .65 "#4c9b99", 1 "#fff5c4")']
  for i,(label,path) in enumerate(cases):
   with wave.open(str(path)) as w:
    rate=w.getframerate(); channels=w.getnchannels(); assert w.getsampwidth()==2
    x=np.frombuffer(w.readframes(w.getnframes()),dtype='<i2').astype(float).reshape(-1,channels).mean(axis=1)/32768
   y=signal.resample_poly(x,24000,rate)
   f,t,z=signal.stft(y,24000,nperseg=480,noverlap=360,nfft=1024,boundary=None,padded=False)
   db=20*np.log10(np.maximum(2*np.abs(z),1e-6)); rms=np.sqrt(np.mean(np.lib.stride_tricks.sliding_window_view(y,480)[::120]**2,axis=1)); rt=(np.arange(len(rms))*120+240)/24000
   active=rms>max(float(rms.max())*.1,.0001)
   ix=np.where(active)[0]; spans=[]
   if len(ix):
    chunks=np.split(ix,np.where(np.diff(ix)>1)[0]+1)
    spans=[[round(float(rt[c[0]]-.01),4),round(float(rt[c[-1]]+.01),4)] for c in chunks if len(c)>=3]
   trajectory,poles=tracks(y,24000)
   # Hide low-level/silent-frame roots; extrapolated roots there are not speech evidence.
   poles=poles[np.interp(poles[:,0],rt,rms)>max(float(rms.max())*.05,.0001)]
   stem=TMP/f'{group}-{i}'; sp=stem.with_suffix('.tsv'); lp=Path(str(stem)+'-poles.tsv'); wp=Path(str(stem)+'-rms.tsv')
   with sp.open('w') as o:
    for ti,time in enumerate(t):
     for fi,freq in enumerate(f):
      if freq<=4500:o.write(f'{time:.5f}\t{freq:.1f}\t{db[fi,ti]:.3f}\n')
     o.write('\n')
   np.savetxt(lp,poles); np.savetxt(wp,np.c_[rt,rms])
   report['cases'][label]={'path':str(path.relative_to(ROOT)),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'rate':rate,'analysis_channels':'mean of source channels','source_channels':channels,'duration':len(x)/rate,'peak':float(abs(x).max()),'active_level_gate_intervals':spans,'candidate_poles_file':str(lp.relative_to(ROOT))}
   # Preserve compact resonance summaries in fixed windows, with no automatic phoneme labels.
   if len(ix):
    onset=rt[ix[0]]-.01; summaries={}
    for title,left,right in [('onset',onset+.015,onset+.065),('following',onset+.10,onset+.18),('late',rt[ix[-1]]-.09,rt[ix[-1]]-.02)]:
     summaries[title]={'seconds':[float(left),float(right)],'pole2_by_order':{str(order):float(np.median(v)) if len(v:=poles[(poles[:,0]>=left)&(poles[:,0]<=right)&(poles[:,1]==order)&(poles[:,2]==2),3]) else None for order in (12,14,16)}}
    report['cases'][label]['descriptive_windows']=summaries
   commands += [f"set title '{label}: original time / shared dBFS'",f'set xrange [0:{len(x)/rate}]','set yrange [0:4500]','set cbrange [-70:-10]','set colorbox',f"plot '{sp}' u 1:2:3 w image notitle",f"set title '{label}: RMS and LPC poles (orders 12/14/16)'",'unset colorbox','set yrange [0:3500]','set y2range [0:0.75]',f"plot '{lp}' u 1:4 w p pt 7 ps .18 notitle, '{wp}' u 1:2 axes x1y2 w l lc rgb '#cc5555' notitle"]
  commands+=['unset multiplot']; gp=TMP/(group+'.gnuplot');gp.write_text('\n'.join(commands));subprocess.run(['gnuplot',str(gp)],check=True)
 (OUT/'measurements.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
 print(json.dumps({k:{'duration':v['duration'],'active':v['active_level_gate_intervals'],'windows':v.get('descriptive_windows')} for k,v in report['cases'].items()},ensure_ascii=False))
if __name__=='__main__':main()
