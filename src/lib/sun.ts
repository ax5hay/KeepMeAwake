/**
 * Geometry for the horizon scene. Pure functions over a fixed viewBox so the
 * component stays declarative and the maths is testable.
 *
 * The sun travels a quarter circle: zenith at the start of a timed session,
 * the right-hand horizon when it ends. Indefinite sessions hold it at zenith;
 * when off it sits just below the line with only its crown showing.
 */
import type { Mode } from './types';

export const SCENE = {
  width: 380,
  height: 240,
  horizonY: 168,
  centerX: 190,
  radius: 112,
  sunRadius: 24,
  /** How far below the horizon the sun's centre sits when off. */
  setDepth: 18,
} as const;

export interface Point {
  x: number;
  y: number;
}

/** Centre of the sun for a given mode and 0..1 progress. */
export function sunPosition(mode: Mode, progressValue: number): Point {
  const { centerX, horizonY, radius, setDepth } = SCENE;
  if (mode === 'off') return { x: centerX, y: horizonY + setDepth };
  // Always, and a watched condition: no end in sight, so the sun stands still.
  if (mode === 'indefinite' || mode === 'watching') return { x: centerX, y: horizonY - radius };
  const p = Math.min(1, Math.max(0, progressValue));
  const phi = (p * Math.PI) / 2;
  return { x: centerX + radius * Math.sin(phi), y: horizonY - radius * Math.cos(phi) };
}

/** Where the sun will set. */
export function settingPoint(): Point {
  return { x: SCENE.centerX + SCENE.radius, y: SCENE.horizonY };
}

/** SVG path from the sun's current position along its arc to the horizon. */
export function remainingArcPath(from: Point): string {
  const end = settingPoint();
  const r = SCENE.radius;
  return `M ${round(from.x)} ${round(from.y)} A ${r} ${r} 0 0 1 ${end.x} ${end.y}`;
}

function round(n: number): number {
  return Math.round(n * 100) / 100;
}
