import { invoke } from "@tauri-apps/api/core";
import { computed, ref } from "vue";

export const desktopPlatform = ref<string | null>(null);
export const supportsScreenCapture = computed(() => desktopPlatform.value !== null && desktopPlatform.value !== "macos");

let platformPromise: Promise<string> | undefined;

// Ask the native process so a WebView user agent cannot select the wrong API.
export const getDesktopPlatform = () => {
  platformPromise ??= invoke<string>("desktop_platform").then((platform) => {
    desktopPlatform.value = platform;
    return platform;
  }).catch((error) => {
    platformPromise = undefined;
    throw error;
  });
  return platformPromise;
};
