#!/usr/bin/env python3
"""Offline exact-pair ending inspection; no models or generation."""
import os
for name in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS','MKL_NUM_THREADS'):os.environ[name]='2'
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave
sys.dont_write_bytecode=True
import numpy as np
from scipy import signal
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'research/artifacts/omnivoice-duration-control-v1'
SCRATCH=ROOT/'.local/omnivoice-audition/duration-control-v1/analysis'
FILES={
 'original':(ROOT/'research/artifacts/omnivoice-audition-v1/OVA01.wav','043724bb0ec7ab206fa5a4769ad2e23d6cdb2f5a8e13ea3fe1d226c378618f3a'),
 'duration45':(OUT/'OVA01-duration45.wav','3bed1d92a6a1abcb43c578fe40fe0dc616b4cf62e7936868f53b15cbe98b4b46'),
}

def main():
    SCRATCH.mkdir(exist_ok=True)
    summary={'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'scope':'Comparison of actual complete generated waveforms; no time stretching or silence padding',
        'methods':{'spectrogram':'20ms Hann,5ms hop,1024FFT,24kHz,2|STFT| dBFS,no gain normalization',
            'end_metrics':'Unweighted original PCM RMS/peak in last20/50/100ms; measurements not speech/silence classifier'},'takes':{}}
    for name,(path,expected) in FILES.items():
        if hashlib.sha256(path.read_bytes()).hexdigest()!=expected:raise ValueError('Original changed')
        with wave.open(str(path),'rb') as w:
            assert (w.getframerate(),w.getnchannels(),w.getsampwidth())==(24000,1,2)
            frames=w.getnframes();pcm=w.readframes(frames);x=np.frombuffer(pcm,dtype='<i2').astype(float)/32768
        end={}
        for ms in (20,50,100):
            y=x[-ms*24:];end[str(ms)]={'rms_dbfs':float(20*np.log10(max(np.sqrt(np.mean(y*y)),1e-8))),'peak':float(np.max(np.abs(y)))}
        summary['takes'][name]={'source_path':str(path.relative_to(ROOT)),'sha256':expected,'frames':frames,
            'duration_seconds':frames/24000,'peak':float(np.max(np.abs(x))),'full_scale_samples':int(np.sum(np.abs(x)>=32767/32768)),
            'last_sample':float(x[-1]),'ending_windows_ms':end}
        # Source-guided visible periodic onset, selected after the shared plot.
        # This interval ends at the file boundary, not a verified vowel offset.
        start=79800 if name=='original' else 105000
        summary['takes'][name]['last_vowel_like_periodic_portion']={
            'start_frame':start,'end_frame':frames,'visible_duration_to_boundary_ms':(frames-start)/24,
            'onset_sensitivity_ms':10,'offset_is_file_boundary':True,
            'interpretation':'Exploratory visible interval, not full phoneme duration or proof of complete final vowel'}
        tail=x[-14400:]
        np.savetxt(SCRATCH/(name+'-tail.tsv'),np.column_stack(((np.arange(len(tail))-len(tail))/24000,tail)),delimiter='\t')
        f,t,z=signal.stft(x,24000,window='hann',nperseg=480,noverlap=360,nfft=1024,boundary=None,padded=False)
        db=20*np.log10(np.maximum(2*np.abs(z),1e-6))
        with (SCRATCH/(name+'-spectrum.tsv')).open('w') as out:
            for i,time in enumerate(t):
                if time<frames/24000-.6:continue
                for j,freq in enumerate(f):
                    if freq<=4500:out.write(f'{time-frames/24000:.6f}\t{freq:.4f}\t{db[j,i]:.5f}\n')
                out.write('\n')
    commands=["set terminal pngcairo size 1600,1050 enhanced font 'Arial,14'",f"set output '{OUT}/ending-comparison.png'",
        "set multiplot layout 2,2 rowsfirst title 'Same OVA01 request; default 3.40 s vs explicit 4.50 s (grid output 4.52 s)'",
        "set xrange [-.6:0]","set xlabel 'Seconds before original file end'","set palette defined (0 '#07101e', .3 '#254e70', .55 '#4c9b99', .8 '#e3b75e', 1 '#fff5c4')"]
    for name in FILES:
        commands += [f"set title '{name}: final 600 ms'","set yrange [0:4500]","set ylabel 'Frequency (Hz)'",
            "set cbrange [-70:-10]","set colorbox",f"plot '{SCRATCH}/{name}-spectrum.tsv' u 1:2:3 with image notitle"]
    for name in FILES:
        commands += [f"set title '{name}: original PCM'","unset colorbox","set yrange [-1:1]","set ylabel 'Amplitude'","set grid xtics",
            f"plot '{SCRATCH}/{name}-tail.tsv' u 1:2 with lines lw 1 lc rgb '#257c86' notitle"]
    commands.append('unset multiplot')
    script=SCRATCH/'compare.gnuplot';script.write_text('\n'.join(commands)+'\n')
    subprocess.run(['gnuplot',str(script)],check=True)
    (OUT/'measurements.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(summary,ensure_ascii=False))

if __name__=='__main__':main()
