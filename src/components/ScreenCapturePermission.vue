<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getDesktopPlatform } from "../platform";
import { showToast } from "../store";

const needsPermission = ref(false);
const checking = ref(false);
let timer: number | undefined;

const checkPermission = async () => {
  needsPermission.value = !(await invoke<boolean>("screen_capture_permission"));
};

const requestAccess = async () => {
  checking.value = true;
  try {
    await invoke("request_screen_capture_permission");
    await checkPermission();
  } catch (error) {
    showToast(`无法请求屏幕录制权限：${String(error)}`, "error");
  } finally {
    checking.value = false;
  }
};

onMounted(async () => {
  try {
    if (await getDesktopPlatform() !== "macos") return;
    await checkPermission();
    timer = window.setInterval(() => {
      checkPermission().catch(console.error);
    }, 5000);
  } catch (error) {
    showToast(`无法检查屏幕录制权限：${String(error)}`, "error");
  }
});

onUnmounted(() => window.clearInterval(timer));
</script>

<template>
  <div v-if="needsPermission" role="status" class="fixed bottom-6 left-6 right-6 z-180 mx-auto max-w-xl rounded-2xl border border-border-main bg-bg-card p-5 shadow-xl">
    <p class="font-bold mb-2">需要屏幕录制权限</p>
    <p class="text-sm text-text-sec">请在「系统设置 → 隐私与安全性 → 屏幕录制（或屏幕与系统音频录制）」中允许 ChronoSnap。授权前自动截图暂不运行，其他功能可继续使用。系统若提示重新打开应用，请先退出再打开。</p>
    <button :disabled="checking" class="mt-3 rounded-xl bg-[#007AFF] px-4 py-2 text-sm font-bold text-white disabled:opacity-50" @click="requestAccess">{{ checking ? '正在检查…' : '请求屏幕录制权限' }}</button>
  </div>
</template>
