import { invoke } from "@tauri-apps/api/core";

let platformPromise: Promise<string> | undefined;

// Ask the native process so a WebView user agent cannot select the wrong API.
export const getDesktopPlatform = () => {
  platformPromise ??= invoke<string>("desktop_platform").catch((error) => {
    platformPromise = undefined;
    throw error;
  });
  return platformPromise;
};
