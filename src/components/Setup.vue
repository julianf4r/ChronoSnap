<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Settings, FolderOpen, Plus } from "lucide-vue-next";
import { savePath, dbPath, isSetupComplete, captureInterval, retainDays, theme } from "../store";
import { load as loadStore } from "@tauri-apps/plugin-store";

const emit = defineEmits(["complete"]);

const selectFolder = async () => { const s = await open({ directory: true }); if (s) savePath.value = s as string; };
const selectDBFile = async () => { const s = await open({ filters: [{ name: "SQLite", extensions: ["db"] }] }); if (s) dbPath.value = s as string; };
const createDBFile = async () => { const s = await save({ filters: [{ name: "SQLite", extensions: ["db"] }] }); if (s) dbPath.value = s as string; };

const completeSetup = async () => {
  if (savePath.value && dbPath.value) {
    await invoke("update_db_path", { path: dbPath.value });
    await invoke("update_interval", { seconds: captureInterval.value });

    const store = await loadStore("config.json");
    await store.set("savePath", savePath.value);
    await store.set("dbPath", dbPath.value);
    await store.set("retainDays", retainDays.value);
    await store.set("captureInterval", captureInterval.value);
    await store.set("theme", theme.value);
    await store.save();

    isSetupComplete.value = true;
    emit("complete");
  }
};
</script>

<template>
  <div class="flex-1 flex items-center justify-center p-10 bg-bg-main h-screen">
    <div class="bg-bg-card p-12 rounded-4xl shadow-2xl max-w-lg text-center border border-border-main">
      <Settings :size="40" class="text-[#007AFF] mx-auto mb-8" />
      <h1 class="text-3xl font-bold mb-4">初始化设置</h1>
      <div class="space-y-6 text-left mb-10">
        <div class="space-y-2">
          <label class="text-xs font-bold text-text-sec">截图保存目录</label>
          <div class="flex gap-2">
            <input type="text" readonly :value="savePath" placeholder="请选择目录..." class="flex-1 bg-bg-input rounded-xl px-4 py-2.5 text-sm outline-none" />
            <button @click="selectFolder" class="bg-bg-card border border-border-main p-2.5 rounded-xl hover:border-[#007AFF] transition-all"><FolderOpen :size="18" /></button>
          </div>
        </div>
        <div class="space-y-2">
          <label class="text-xs font-bold text-text-sec">数据库文件</label>
          <div class="flex gap-2">
            <input type="text" readonly :value="dbPath" placeholder="请选择或创建数据库文件..." class="flex-1 bg-bg-input rounded-xl px-4 py-2.5 text-sm outline-none" />
            <div class="flex gap-1">
              <button @click="selectDBFile" class="bg-bg-card border border-border-main p-2.5 rounded-xl hover:border-[#007AFF]"><FolderOpen :size="18" /></button>
              <button @click="createDBFile" class="bg-bg-card border border-border-main p-2.5 rounded-xl hover:border-[#007AFF]"><Plus :size="18" /></button>
            </div>
          </div>
        </div>
      </div>
      <button @click="completeSetup" :disabled="!savePath || !dbPath" class="bg-[#007AFF] text-white px-8 py-4 rounded-2xl font-semibold w-full disabled:opacity-50 transition-all active:scale-95">开始使用</button>
    </div>
  </div>
</template>
