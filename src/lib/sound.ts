/**
 * The two notes the sun makes, if asked to: up as it rises, down as it sets.
 * Made here from two sine tones, so there is no sound file to ship. Nothing
 * is set up until the first note is wanted, and only an open window can
 * play one.
 */

let context: AudioContext | null = null;

/** The pitches of a note, in hertz: a fifth, rising or falling. */
export function notes(rising: boolean): [number, number] {
  return rising ? [392, 587.33] : [587.33, 392];
}

/** Plays the rise or the set, quietly. Does nothing where there is no audio. */
export function chime(rising: boolean): void {
  try {
    context ??= new AudioContext();
    const audio = context;
    void audio.resume();
    notes(rising).forEach((pitch, i) => {
      const start = audio.currentTime + i * 0.11;
      const tone = audio.createOscillator();
      const level = audio.createGain();
      tone.type = 'sine';
      tone.frequency.value = pitch;
      level.gain.setValueAtTime(0, start);
      level.gain.linearRampToValueAtTime(0.07, start + 0.015);
      level.gain.exponentialRampToValueAtTime(0.0001, start + 0.42);
      tone.connect(level).connect(audio.destination);
      tone.start(start);
      tone.stop(start + 0.45);
    });
  } catch {
    // No audio device, or none allowed: the sun rises in silence.
  }
}
