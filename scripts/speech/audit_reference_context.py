#!/usr/bin/env python3
"""Offline RQ06 context inspection, tied to its original WAV; no model calls."""
import os
for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS','MKL_NUM_THREADS'):
    os.environ[key]='2'
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave
sys.dont_write_bytecode=True
import numpy as np
from scipy import signal
from audit_silero_stress import tracks

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'research/artifacts/reference-quality-slot2'
SCRATCH=ROOT/'.local/reference-quality/slot2/analysis'
SHA='b2eba78877afec1cb6a038617bbc4f037b34f89afb1a0ac05a5b43d14893c476'
# Selected from the full and zoomed spectral plots, not word/phone alignment.
# The low-frequency nasal portions between candidate vowels are excluded.
REGIONS={
    'first_phrase_ending':{'intended_meaning':'lock','replay':(1.0,1.8),'vowels':((1.060,1.110),(1.250,1.535))},
    'second_phrase_ending':{'intended_meaning':'castle','replay':(4.1,4.9),'vowels':((4.215,4.255),(4.335,4.640))},
}

def main():
    SCRATCH.mkdir(exist_ok=True)
    path=OUT/'RQ06.wav'
    if hashlib.sha256(path.read_bytes()).hexdigest()!=SHA: raise ValueError('Original changed')
    with wave.open(str(path),'rb') as w:
        rate,frames=w.getframerate(),w.getnframes()
        assert (rate,w.getnchannels(),w.getsampwidth())==(24000,1,2)
        pcm=w.readframes(frames)
    x=np.frombuffer(pcm,dtype='<i2').astype(float)/32768
    f,t,z=signal.stft(x,rate,window='hann',nperseg=480,noverlap=360,nfft=1024,boundary=None,padded=False)
    db=20*np.log10(np.maximum(2*np.abs(z),1e-6))
    with (SCRATCH/'context-spectrum.tsv').open('w') as out:
        for i,time in enumerate(t):
            for j,freq in enumerate(f):
                if freq<=4500: out.write(f'{time:.6f}\t{freq:.4f}\t{db[j,i]:.5f}\n')
            out.write('\n')
    trajectory,poles=tracks(x,rate)
    np.savetxt(SCRATCH/'context-tracks.tsv',trajectory,delimiter='\t')
    np.savetxt(SCRATCH/'context-lpc.tsv',poles,delimiter='\t')
    commands=["set terminal pngcairo size 1800,1100 enhanced font 'Arial,14'",f"set output '{OUT}/context-inspection.png'",
        "set multiplot layout 3,1 title 'RQ06 contextual homographs · original time, no forced alignment'",
        "set xrange [0:4.96]", "set yrange [0:4500]", "set cbrange [-70:-10]",
        "set palette defined (0 '#07101e', .3 '#254e70', .55 '#4c9b99', .8 '#e3b75e', 1 '#fff5c4')",
        "set xlabel 'Original time (s)'", "set ylabel 'Frequency (Hz)'", "set cblabel 'Sinusoid-relative dBFS'",
        f"plot '{SCRATCH}/context-spectrum.tsv' u 1:2:3 with image notitle",
        "unset colorbox", "set yrange [-60:-10]", "set y2range [80:400]", "set y2tics", "set ylabel 'RMS (dBFS)'", "set y2label 'F0 estimate (Hz)'", "set key bottom right",
        f"plot '{SCRATCH}/context-tracks.tsv' u 1:2 w l lw 2 lc rgb '#257c86' t 'RMS', '' u 1:3 axes x1y2 w p pt 7 ps .4 lc rgb '#993366' t 'F0 candidate'",
        "unset y2tics", "unset y2label", "set yrange [150:3500]", "set ylabel 'LPC candidate pole (Hz)'", "set key top right",
        f"plot '{SCRATCH}/context-lpc.tsv' u 1:($2==12?$4:1/0) w p pt 6 ps .3 t '12', '' u 1:($2==14?$4:1/0) w p pt 7 ps .3 t '14', '' u 1:($2==16?$4:1/0) w p pt 4 ps .3 t '16'", "unset multiplot"]
    script=SCRATCH/'context.gnuplot'
    script.write_text('\n'.join(commands)+'\n')
    subprocess.run(['gnuplot',str(script)],check=True)
    target_commands=["set terminal pngcairo size 1600,1000 enhanced font 'Arial,14'",f"set output '{OUT}/context-targets.png'",
        "set multiplot layout 2,2 rowsfirst title 'RQ06 candidate phrase-final words · source-guided windows, not forced alignment'",
        "set palette defined (0 '#07101e', .3 '#254e70', .55 '#4c9b99', .8 '#e3b75e', 1 '#fff5c4')"]
    for name,region in REGIONS.items():
        left,right=region['replay']
        target_commands += [f"set xrange [{left}:{right}]",f"set title '{name}: intended {region['intended_meaning']}'",
            "set yrange [0:4500]","set cbrange [-70:-10]","set colorbox","set xlabel 'Original time (s)'","set ylabel 'Frequency (Hz)'","unset object"]
        for index,(start,end) in enumerate(region['vowels'],1):
            target_commands.append(f"set object {index} rect from {start},graph 0 to {end},graph 1 front fillcolor rgb '#d89b3e' fillstyle transparent solid 0.12 noborder")
        target_commands.append(f"plot '{SCRATCH}/context-spectrum.tsv' u 1:2:3 with image notitle")
    for name,region in REGIONS.items():
        left,right=region['replay']
        target_commands += ["unset object",f"set xrange [{left}:{right}]",f"set title '{name}: LPC order sensitivity'",
            "unset colorbox","set yrange [150:3000]","set ylabel 'Candidate pole (Hz)'",
            f"plot '{SCRATCH}/context-lpc.tsv' u 1:($2==12?$4:1/0) w p pt 6 ps .5 t '12', '' u 1:($2==14?$4:1/0) w p pt 7 ps .4 t '14', '' u 1:($2==16?$4:1/0) w p pt 4 ps .5 t '16'"]
    target_commands.append('unset multiplot')
    script=SCRATCH/'targets.gnuplot'
    script.write_text('\n'.join(target_commands)+'\n')
    subprocess.run(['gnuplot',str(script)],check=True)
    candidates={}
    for name,region in REGIONS.items():
        left,right=(round(seconds*rate) for seconds in region['replay'])
        crop_path=OUT/('RQ06-'+name+'.wav')
        with wave.open(str(crop_path),'wb') as crop:
            crop.setparams((1,2,rate,right-left,'NONE','not compressed'))
            crop.writeframes(pcm[left*2:right*2])
        with wave.open(str(crop_path),'rb') as crop:
            assert crop.readframes(crop.getnframes())==pcm[left*2:right*2]
        candidate={'intended_meaning':region['intended_meaning'],'replay':{'path':str(crop_path.relative_to(ROOT)),
            'sha256':hashlib.sha256(crop_path.read_bytes()).hexdigest(),'source_span':{'start_frame':left,'end_frame':right},
            'coordinate_system':'original_frames_half_open','pcm_byte_exact':True,'localization':'source-guided padded phrase ending, not certified word alignment'},'candidate_vowels':[]}
        for start,end in region['vowels']:
            selected=trajectory[(trajectory[:,0]>=start)&(trajectory[:,0]<=end)]
            f0=selected[:,2]; f0=f0[np.isfinite(f0)]
            candidate['candidate_vowels'].append({'source_span':{'start_frame':round(start*rate),'end_frame':round(end*rate)},
                'duration_ms':round((end-start)*1000,3),'boundary_sensitivity_ms':[round((end-start)*1000-20,3),round((end-start)*1000+20,3)],
                'boundary_sensitivity_note':'Independent ±10ms onset/offset perturbation, not a confidence interval',
                'median_rms_dbfs':float(np.median(selected[:,1])),
                'median_f0_candidate_hz':float(np.median(f0)) if len(f0) else None})
        candidates[name]=candidate
    result={'source_sha256':SHA,'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'tracks_dependency_sha256':hashlib.sha256((ROOT/'scripts/speech/audit_silero_stress.py').read_bytes()).hexdigest(),
        'methods':{'spectrogram':'24kHz;20ms Hann;5ms hop;1024FFT;2|STFT| sinusoid-relative dBFS;no normalization',
          'rms':'20ms;5ms hop','f0':'YIN-style;80–400Hz;30ms fixed difference region;threshold .15;not probability',
          'lpc':'12kHz;30ms Hamming;preemphasis.97;orders12/14/16;candidate poles not confirmed formants'},
        'frames':frames,'duration_seconds':frames/rate,'peak':float(np.max(np.abs(x))),
        'full_scale_samples':int(np.sum(np.abs(x)>=32767/32768)),
        'human_listening':None,'word_alignment':None,'target_contrast_status':'suspect',
        'stress_verdict':'Unresolved: both candidate words have short V1/long V2; intended context-driven stress reversal is not demonstrated',
        'candidate_regions':candidates}
    (OUT/'context-measurements.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')

if __name__=='__main__': main()
