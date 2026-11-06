#!/usr/bin/env python3
"""CPU-only descriptive inspection of the frozen three-take OmniVoice audition."""
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
sys.path.insert(0,str(ROOT/'scripts/speech'))
from audit_silero_stress import tracks
from asr_review import lexical_tokens,token_differences

OUT=ROOT/'research/artifacts/omnivoice-audition-v1'
SCRATCH=ROOT/'.local/omnivoice-audition/analysis'
EXPECTED={'OVA01':'043724bb0ec7ab206fa5a4769ad2e23d6cdb2f5a8e13ea3fe1d226c378618f3a',
 'OVA02':'989a08c41ff22b2716d09b90fc500f882f33c9caf46ccb4374c2be5beab3eb6f',
 'OVA03':'da2b9dffc21c8df4fcd98229fa0d93ae463f87a45e4c18c8ee6fa37923992048'}

def main():
    SCRATCH.mkdir(exist_ok=True)
    summary={'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':{},
        'scope':'Descriptive fixed original recordings; ASR lexical screens are not phonetic grades',
        'spectrogram':'20ms Hann,5ms hop,1024FFT,24kHz,2|STFT| sinusoid-relative dBFS,no normalization',
        'lpc':'12kHz,30ms Hamming,preemphasis.97,orders12/14/16; candidate resonances, not confirmed formants'}
    for case,expected in EXPECTED.items():
        source=OUT/(case+'.wav')
        if hashlib.sha256(source.read_bytes()).hexdigest()!=expected:raise ValueError('Original changed')
        with wave.open(str(source),'rb') as w:
            assert (w.getframerate(),w.getnchannels(),w.getsampwidth())==(24000,1,2)
            x=np.frombuffer(w.readframes(w.getnframes()),dtype='<i2').astype(float)/32768
        s=json.loads((OUT/(case+'.json')).read_text());review=json.loads((OUT/(case+'-review.json')).read_text())
        words='У меня сегодня два вопроса, пять идей и двадцать одна свободная минута.' if case=='OVA03' else s['source_text']
        summary['cases'][case]={'sha256':expected,'frames':len(x),'peak':float(np.max(np.abs(x))),
            'full_scale_samples':int(np.sum(np.abs(x)>=32767/32768)),
            'last20ms_rms_dbfs':float(20*np.log10(max(np.sqrt(np.mean(x[-480:]**2)),1e-8))),
            'last20ms_peak':float(np.max(np.abs(x[-480:]))),'last_sample':float(x[-1]),
            'source_word_count':len(lexical_tokens(words)),'whole_clip_words_per_minute':len(lexical_tokens(words))*60/(len(x)/24000),
            'rate_note':'Whole-clip rate, not articulation rate; numbers counted as expected spoken words',
            'expected_spoken_words':words,'literal_transcript':review['literal_transcript'],
            'lexical_differences':token_differences(words,review['literal_transcript'])}
        tail=x[-4800:]
        np.savetxt(SCRATCH/(case+'-tail.tsv'),np.column_stack(((np.arange(len(tail))-len(tail))/24000,tail)),delimiter='\t')
        if case!='OVA02':continue
        f,t,z=signal.stft(x,24000,window='hann',nperseg=480,noverlap=360,nfft=1024,boundary=None,padded=False)
        db=20*np.log10(np.maximum(2*np.abs(z),1e-6))
        with (SCRATCH/'contrast-spectrum.tsv').open('w') as out:
            for i,time in enumerate(t):
                for j,freq in enumerate(f):
                    if freq<=4500:out.write(f'{time:.6f}\t{freq:.4f}\t{db[j,i]:.5f}\n')
                out.write('\n')
        trajectory,poles=tracks(x,24000)
        np.savetxt(SCRATCH/'contrast-lpc.tsv',poles,delimiter='\t')
    script=SCRATCH/'contrast.gnuplot'
    script.write_text('\n'.join(["set terminal pngcairo size 1600,900 enhanced font 'Arial,14'",f"set output '{OUT}/contextual-lateral.png'",
        "set multiplot layout 2,1 title 'OmniVoice first take: Это лук, а это люк. · original time'", "set xrange [0:1.84]",
        "set yrange [0:4500]","set cbrange [-70:-10]","set xlabel 'Original time (s)'","set ylabel 'Frequency (Hz)'",
        "set palette defined (0 '#07101e', .3 '#254e70', .55 '#4c9b99', .8 '#e3b75e', 1 '#fff5c4')",
        f"plot '{SCRATCH}/contrast-spectrum.tsv' u 1:2:3 with image notitle", "unset colorbox","set yrange [150:3500]",
        "set ylabel 'Candidate LPC pole (Hz)'", "set title 'Order sensitivity; source-guided localization, no phone alignment'",
        f"plot '{SCRATCH}/contrast-lpc.tsv' u 1:($2==12?$4:1/0) w p pt 6 ps .4 t '12', '' u 1:($2==14?$4:1/0) w p pt 7 ps .35 t '14', '' u 1:($2==16?$4:1/0) w p pt 4 ps .4 t '16'", "unset multiplot"])+'\n')
    subprocess.run(['gnuplot',str(script)],check=True)
    end_commands=["set terminal pngcairo size 1600,900 enhanced font 'Arial,14'",f"set output '{OUT}/ending-inspection.png'",
        "set multiplot layout 3,1 title 'Last 200 ms of each original · same amplitude scale · fixed estimated generation length'",
        "set xrange [-.2:0]","set yrange [-1:1]","set xlabel 'Seconds before original file end'","set ylabel 'PCM amplitude'","set grid xtics"]
    for case in EXPECTED:
        end_commands += [f"set title '{case}'",f"plot '{SCRATCH}/{case}-tail.tsv' u 1:2 w l lw 1 lc rgb '#257c86' notitle"]
    end_commands.append('unset multiplot')
    script=SCRATCH/'endings.gnuplot';script.write_text('\n'.join(end_commands)+'\n')
    subprocess.run(['gnuplot',str(script)],check=True)
    (OUT/'measurements.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(summary,ensure_ascii=False))

if __name__=='__main__':main()
