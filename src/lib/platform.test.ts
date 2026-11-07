import { describe, expect, it } from 'vitest';

import { detect } from './platform';

describe('detect', () => {
  it('knows a Mac by either hint', () => {
    expect(detect({ uaDataPlatform: 'macOS' })).toBe(true);
    // WKWebView has no userAgentData; every Mac says MacIntel, Apple Silicon too.
    expect(detect({ platform: 'MacIntel' })).toBe(true);
  });

  it('prefers userAgentData when both are there', () => {
    expect(detect({ uaDataPlatform: 'Windows', platform: 'MacIntel' })).toBe(false);
  });

  it('is false for Windows and when nothing is known', () => {
    expect(detect({ uaDataPlatform: 'Windows', platform: 'Win32' })).toBe(false);
    expect(detect({ platform: 'Win32' })).toBe(false);
    expect(detect({})).toBe(false);
  });
});
