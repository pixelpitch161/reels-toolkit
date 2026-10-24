"""Band check for #32: each fighter's band (tuba 40-260 Hz, flute 900-6000 Hz) over the rest of the mix, per section
(from the stems audio.py writes to check/).

    python band_check.py"""
import numpy as np, sys
from scipy.io import wavfile
from scipy.signal import butter, sosfilt
def rd(k):
    sr,x=wavfile.read(f'check/stem_{k}.wav'); x=x.astype(float); x=x.mean(1) if x.ndim>1 else x
    return sr, x/32768 if np.abs(x).max()>2 else x
def band(x,sr,lo,hi): return sosfilt(butter(4,[lo,hi],'band',fs=sr,output='sos'),x)
spans=[(1.9,8),(8,20),(20,32),(32,44),(44,51.5)]
def db(x): return 20*np.log10(np.sqrt((x**2).mean())+1e-9)
for fig, lo, hi, bedk in (('tuba',40,260,('drums','brass','pad','musicbox','fx')),('flute',900,6000,('drums','brass','pad','musicbox','fx'))):
    sr,f=rd(fig); f=band(f,sr,lo,hi)
    bed=sum(band(rd(k)[1],sr,lo,hi) for k in bedk)
    print(fig, f'{lo}-{hi} Hz over the bed:', ' '.join(f'{db(f[int(a*sr):int(b*sr)])-db(bed[int(a*sr):int(b*sr)]):+5.1f}' for a,b in spans))
