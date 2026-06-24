/** Tauri 桌面端平台识别（用于标题栏等原生 UI 分支） */
export type AppPlatform = 'macos' | 'windows' | 'linux' | 'other';

export function detectAppPlatform(): AppPlatform {
  const ua = navigator.userAgent;
  const platform = navigator.platform || '';

  // macOS WebView（WKWebView）UA 含 Macintosh；排除误报
  if (
    (/Macintosh|Mac OS X|MacIntel/i.test(ua) || /^Mac/i.test(platform)) &&
    !/Windows/i.test(ua)
  ) {
    return 'macos';
  }

  if (/Windows/i.test(ua) || /Win32|Win64|Windows/i.test(platform)) {
    return 'windows';
  }

  if (/Linux/i.test(ua) || /Linux|X11/i.test(platform)) {
    return 'linux';
  }

  return 'other';
}

/** macOS 使用左侧交通灯标题栏 */
export function useMacTitlebar(platform: AppPlatform = detectAppPlatform()): boolean {
  return platform === 'macos';
}

/** Windows / Linux / 其他平台使用右侧窗口按钮 */
export function useWinStyleTitlebar(platform: AppPlatform = detectAppPlatform()): boolean {
  return platform !== 'macos';
}
