#!/usr/bin/env python3
"""Offline shared-scale inspection of the exact RQ02/RQ03 recordings."""
import os
for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','VECLIB_MAXIMUM_THREADS','MKL_NUM_THREADS'):
    os.environ[key] = '2'
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave
sys.dont_write_bytecode = True
import numpy as np
from scipy import signal
from audit_silero_stress import tracks

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'research/artifacts/reference-quality-slot1'
SCRATCH = ROOT / '.local/reference-quality/slot1/analysis'
HASHES = {'RQ02':'bf19ea6c569402b083cd07050b8fa901530fc9a387154b18d83278088b496fd1',
          'RQ03':'2cdd4b1197b6c638080b5e8a2a32438b0a7bcff4073999abbaac649d8d066669'}

def main():
    SCRATCH.mkdir(exist_ok=True)
    summary = {'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'scope':'Two exact synthetic words; descriptive signal comparison, no calibrated pronunciation classifier',
        'methods':{'spectrogram':'20 ms Hann, 5 ms hop, 1024 FFT, 24 kHz, 2|STFT| sinusoid-relative dBFS, no gain normalization',
          'lpc':'12 kHz, 30 ms Hamming, preemphasis .97, autocorrelation orders 12/14/16; candidate poles, not confirmed formants',
          'tracks_dependency_sha256':hashlib.sha256((ROOT/'scripts/speech/audit_silero_stress.py').read_bytes()).hexdigest()}, 'cases':{}}
    for case, expected in HASHES.items():
        path = OUT / (case+'.wav')
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected: raise ValueError('Exact fixture changed')
        with wave.open(str(path),'rb') as w:
            rate, frames = w.getframerate(), w.getnframes()
            assert (rate,w.getnchannels(),w.getsampwidth()) == (24000,1,2)
            pcm = w.readframes(frames)
            x = np.frombuffer(pcm,dtype='<i2').astype(float)/32768
        f,t,z = signal.stft(x, rate, window='hann',nperseg=480,noverlap=360,nfft=1024,boundary=None,padded=False)
        db = 20*np.log10(np.maximum(2*np.abs(z),1e-6))
        with (SCRATCH/(case+'-spectrogram.tsv')).open('w') as output:
            for i,time in enumerate(t):
                for j,freq in enumerate(f):
                    if freq <=4500: output.write(f'{time:.6f}\t{freq:.4f}\t{db[j,i]:.5f}\n')
                output.write('\n')
        trajectory,poles = tracks(x,rate)
        np.savetxt(SCRATCH/(case+'-tracks.tsv'),trajectory,delimiter='\t')
        np.savetxt(SCRATCH/(case+'-lpc.tsv'),poles,delimiter='\t')
        regions = {}
        # Selected after viewing the full shared-scale plot. These are descriptive
        # windows, not automatically segmented phone boundaries or trained thresholds.
        for label,(left,right) in {'early_transition':(.04,.09),'later_vowel':(.18,.22)}.items():
            by_order = {}
            for order in (12,14,16):
                selected = poles[(poles[:,0]>=left)&(poles[:,0]<=right)&(poles[:,1]==order)&(poles[:,2]==2),3]
                by_order[str(order)] = {'median_hz':float(np.median(selected)),'count':len(selected)}
            regions[label] = {'span_seconds':[left,right], 'candidate_second_pole':by_order}
        sensitivity = []
        for shift in (-.01,0,.01):
            for order in (12,14,16):
                selected = poles[(poles[:,0]>=.04+shift)&(poles[:,0]<=.09+shift)&(poles[:,1]==order)&(poles[:,2]==2),3]
                sensitivity.append({'window_shift_seconds':shift,'order':order,'median_hz':float(np.median(selected))})
        regions['early_transition']['window_sensitivity'] = sensitivity
        crop_left,crop_right = 360,5760
        crop_path = OUT/(case+'-transition.wav')
        with wave.open(str(crop_path),'wb') as crop:
            crop.setparams((1,2,rate,crop_right-crop_left,'NONE','not compressed'))
            crop.writeframes(pcm[crop_left*2:crop_right*2])
        with wave.open(str(crop_path),'rb') as crop:
            assert crop.readframes(crop.getnframes()) == pcm[crop_left*2:crop_right*2]
        summary['cases'][case] = {'sha256':expected,'frames':frames,'duration_seconds':frames/rate,
            'peak':float(np.max(np.abs(x))),'full_scale_samples':int(np.sum(np.abs(x)>=32767/32768)),
            'descriptive_regions':regions,
            'targeted_replay':{'path':str(crop_path.relative_to(ROOT)),'sha256':hashlib.sha256(crop_path.read_bytes()).hexdigest(),
              'source_span':{'start_frame':crop_left,'end_frame':crop_right},'coordinate_system':'original_frames_half_open',
              'pcm_byte_exact':True,'purpose':'Onset through vowel, excluding final closure/release; full word remains default'}}
    commands = ["set terminal pngcairo size 1600,1000 enhanced font 'Arial,14'",f"set output '{OUT}/lateral-comparison.png'",
      "set multiplot layout 2,2 rowsfirst title 'Exact isolated references: лук / люк · same scales, original time'",
      "set xrange [0:0.65]", "set palette defined (0 '#07101e', .3 '#254e70', .55 '#4c9b99', .8 '#e3b75e', 1 '#fff5c4')"]
    for case,title in [('RQ02','Лук. · onion'),('RQ03','Люк. · hatch')]:
        commands += [f"set title '{case}: {title}'", "set yrange [0:4500]", "set cbrange [-70:-10]",
          "set xlabel 'Original time (s)'", "set ylabel 'Frequency (Hz)'", "set cblabel 'Sinusoid-relative dBFS'", "set colorbox",
          f"plot '{SCRATCH}/{case}-spectrogram.tsv' u 1:2:3 with image notitle"]
    for case in HASHES:
        commands += [f"set title '{case}: candidate LPC resonances (model-order sensitivity)'", "unset colorbox", "set yrange [150:3500]", "set ylabel 'Candidate pole frequency (Hz)'", "set key top right",
          f"plot '{SCRATCH}/{case}-lpc.tsv' u 1:($2==12?$4:1/0) w p pt 6 ps .5 t '12', '' u 1:($2==14?$4:1/0) w p pt 7 ps .4 t '14', '' u 1:($2==16?$4:1/0) w p pt 4 ps .5 t '16'"]
    commands.append('unset multiplot')
    plot = SCRATCH/'pair.gnuplot'
    plot.write_text('\n'.join(commands)+'\n')
    subprocess.run(['gnuplot',str(plot)],check=True)
    (OUT/'pair-measurements.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(summary,ensure_ascii=False))

if __name__ == '__main__': main()
