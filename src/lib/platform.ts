// The webview's user agent is the one platform signal available without
// pulling in @tauri-apps/plugin-os: WebView2 reports "Windows NT",
// WKWebView "Macintosh", and WebKitGTK "Linux". Only used for cosmetic
// hints like example paths, so a wrong guess is harmless.
//
// Safe to read at module scope: the app runs as an SPA (`ssr = false`),
// so this never evaluates in Node.

export type Platform = "windows" | "macos" | "linux";

export function currentPlatform(): Platform {
  const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Macintosh|Mac OS X/i.test(ua)) return "macos";
  return "linux";
}

const DIRECTORY_PLACEHOLDERS: Record<Platform, string> = {
  windows: "C:\\path\\to\\project",
  macos: "/Users/you/Projects/my-project",
  linux: "/home/you/projects/my-project",
};

/** Example project path to show as a hint, in the local path style. */
export function directoryPlaceholder(): string {
  return DIRECTORY_PLACEHOLDERS[currentPlatform()];
}
