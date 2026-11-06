import { describe, expect, it } from 'vitest';

import { SCENE, remainingArcPath, settingPoint, sunPosition } from './sun';

describe('sunPosition', () => {
  it('rests below the horizon when off', () => {
    const p = sunPosition('off', 0);
    expect(p.x).toBe(SCENE.centerX);
    expect(p.y).toBeGreaterThan(SCENE.horizonY);
    // The crown still peeks over the line.
    expect(p.y - SCENE.sunRadius).toBeLessThan(SCENE.horizonY);
  });

  it('holds zenith when indefinite regardless of progress', () => {
    expect(sunPosition('indefinite', 0.7)).toEqual({
      x: SCENE.centerX,
      y: SCENE.horizonY - SCENE.radius,
    });
  });

  it('travels from zenith to the setting point as a timed session elapses', () => {
    expect(sunPosition('timed', 0)).toEqual({ x: SCENE.centerX, y: SCENE.horizonY - SCENE.radius });
    const end = sunPosition('timed', 1);
    expect(end.x).toBeCloseTo(settingPoint().x);
    expect(end.y).toBeCloseTo(settingPoint().y);
    const mid = sunPosition('timed', 0.5);
    expect(mid.x).toBeGreaterThan(SCENE.centerX);
    expect(mid.y).toBeLessThan(SCENE.horizonY);
  });

  it('clamps progress', () => {
    expect(sunPosition('timed', -1)).toEqual(sunPosition('timed', 0));
    expect(sunPosition('timed', 2).x).toBeCloseTo(sunPosition('timed', 1).x);
  });
});

describe('remainingArcPath', () => {
  it('draws an arc ending at the setting point', () => {
    const path = remainingArcPath({ x: 190, y: 56 });
    expect(path).toBe(`M 190 56 A 112 112 0 0 1 ${settingPoint().x} ${settingPoint().y}`);
  });
});
