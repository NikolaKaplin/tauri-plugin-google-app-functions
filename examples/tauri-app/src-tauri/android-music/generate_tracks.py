"""Generate two original 16-second musical loops, with no external downloads."""
import math
from pathlib import Path
import struct
import wave

RATE = 22050
DURATION = 16
DEST = Path(__file__).parent / 'assets' / 'music'
DEST.mkdir(parents=True, exist_ok=True)
for name, notes, bass in [
    ('morning', [60, 64, 67, 71, 67, 64, 62, 67], [48, 45, 53, 55]),
    ('night', [57, 60, 64, 67, 64, 60, 55, 59], [45, 41, 48, 43]),
]:
    samples = bytearray()
    for i in range(RATE * DURATION):
        t = i / RATE
        beat = t % .5
        midi = notes[int(t * 2) % len(notes)]
        f = 440 * 2 ** ((midi - 69) / 12)
        envelope = min(beat * 100, 1) * math.exp(-beat * 7)
        lead = (math.sin(2 * math.pi * f * t) + .3 * math.sin(4 * math.pi * f * t)) * envelope
        bf = 440 * 2 ** ((bass[int(t / 2) % len(bass)] - 69) / 12)
        low = math.sin(2 * math.pi * bf * t) * (.6 + .4 * math.exp(-beat * 8))
        kick = math.sin(2 * math.pi * (48 * beat + 5 * (1 - math.exp(-beat * 30)))) * math.exp(-beat * 24)
        fade = min(t * 100, 1, (DURATION-t) * 100)
        value = .22 * lead + .16 * low + .13 * kick
        samples.extend(struct.pack('<h', int(max(-1, min(1, value * fade)) * 32767)))
    with wave.open(str(DEST / f'{name}.wav'), 'wb') as audio:
        audio.setparams((1, 2, RATE, 0, 'NONE', 'not compressed'))
        audio.writeframes(samples)
