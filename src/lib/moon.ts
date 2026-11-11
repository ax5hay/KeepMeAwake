/**
 * The moon as it is tonight, from the date alone: no network, no place.
 * Good to about a day, which is all a small drawing can show.
 */

/** Length of a lunation, in days. */
const SYNODIC_DAYS = 29.530588853;

/** A new moon to count from: 6 January 2000, 18:14 UTC. */
const NEW_MOON_MS = Date.UTC(2000, 0, 6, 18, 14);

/** Where the moon is in its month at `ms`: 0 new, 0.5 full, towards 1 waning. */
export function moonPhase(ms: number): number {
  const days = (ms - NEW_MOON_MS) / 86_400_000;
  return (((days % SYNODIC_DAYS) + SYNODIC_DAYS) % SYNODIC_DAYS) / SYNODIC_DAYS;
}

/** How much of the face is lit at `phase`, 0 to 1. */
export function moonLit(phase: number): number {
  return (1 - Math.cos(2 * Math.PI * phase)) / 2;
}

/** What that phase is called. */
export function moonName(phase: number): string {
  const lit = moonLit(phase);
  if (lit < 0.03) return 'New moon';
  if (lit > 0.97) return 'Full moon';
  const half = Math.abs(lit - 0.5) < 0.06;
  if (phase < 0.5) return half ? 'First quarter' : lit < 0.5 ? 'Waxing crescent' : 'Waxing gibbous';
  return half ? 'Last quarter' : lit < 0.5 ? 'Waning crescent' : 'Waning gibbous';
}

/** `Waxing gibbous, 81% lit.`: said when the moon is clicked. */
export function moonLine(phase: number): string {
  return `${moonName(phase)}, ${Math.round(moonLit(phase) * 100)}% lit.`;
}

/**
 * The lit part of a moon of radius `r` centred on the origin, as an SVG
 * path: the lit limb, a half circle, closed by the terminator, half an
 * ellipse. Waxing is lit on the right, as it is seen from the north.
 */
export function moonPath(phase: number, r: number): string {
  const waxing = phase < 0.5;
  const reach = Math.abs(Math.cos(2 * Math.PI * phase)) * r;
  const crescent = moonLit(phase) < 0.5;
  const limb = waxing ? 1 : 0;
  // The terminator bulges towards the lit limb in a crescent, away when gibbous.
  const terminator = waxing === crescent ? 0 : 1;
  const f = (n: number) => Number(n.toFixed(2));
  return `M 0 ${-r} A ${r} ${r} 0 0 ${limb} 0 ${r} A ${f(reach)} ${r} 0 0 ${terminator} 0 ${-r} Z`;
}
