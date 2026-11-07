/**
 * Which OS the window is on. The webview already knows, so there is no IPC:
 * the answer is read once from `navigator` and used for copy, key glyphs and
 * type metrics.
 */

/** What the webview says about the OS it runs on. */
export interface PlatformHints {
  /** `navigator.userAgentData.platform` (`"macOS"`), where the engine has it. */
  uaDataPlatform?: string | undefined;
  /** `navigator.platform`: `"MacIntel"` on every Mac, Apple Silicon included. */
  platform?: string | undefined;
}

/** `true` on macOS. `userAgentData` wins when present; WebView2 has it, WKWebView does not. */
export function detect(hints: PlatformHints): boolean {
  const name = hints.uaDataPlatform || hints.platform || '';
  return /^mac/i.test(name);
}

interface NavigatorWithUaData {
  platform?: string;
  userAgentData?: { platform?: string };
}

function fromNavigator(): PlatformHints {
  const nav = (globalThis as { navigator?: NavigatorWithUaData }).navigator;
  return { uaDataPlatform: nav?.userAgentData?.platform, platform: nav?.platform };
}

/** `true` when this window is on a Mac. */
export const isMac: boolean = detect(fromNavigator());
