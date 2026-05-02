<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { X, FolderOpen, Plus } from "lucide-vue-next";
import { savePath, dbPath, theme, captureInterval, retainDays, showToast } from "../../store";
import { load as loadStore } from "@tauri-apps/plugin-store";
import { StorageHealth } from "../../types";

const emit = defineEmits(['close', 'updated']);

const updateSettings = async () => {
  const health = await invoke<StorageHealth>("check_storage_health", { savePath: savePath.value, dbPath: dbPath.value });
  if (!health.ok) {
    showToast(health.issues[0] || "存储路径不可用", "error");
    emit('updated');
    return;
  }

  const store = await loadStore("config.json");
  await store.set("savePath", savePath.value);
  await store.set("dbPath", dbPath.value);
  await store.set("retainDays", retainDays.value);
  await store.set("captureInterval", captureInterval.value); 
  await store.set("theme", theme.value);
  await store.save();
  await invoke("update_db_path", { path: dbPath.value });
  await invoke("update_interval", { seconds: captureInterval.value });
  emit('updated');
  
  if (theme.value === 'dark') document.documentElement.classList.add('dark');
  else if (theme.value === 'light') document.documentElement.classList.remove('dark');
  else {
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) document.documentElement.classList.add('dark');
    else document.documentElement.classList.remove('dark');
  }
};

const selectFolder = async () => { 
  const s = await open({ directory: true }); 
  if (s) { savePath.value = s as string; await updateSettings(); }
};

const selectDBFile = async () => { 
  const s = await open({ filters: [{ name: "SQLite", extensions: ["db"] }] }); 
  if (s) { dbPath.value = s as string; await updateSettings(); }
};

const createDBFile = async () => { 
  const s = await save({ filters: [{ name: "SQLite", extensions: ["db"] }] }); 
  if (s) { dbPath.value = s as string; await updateSettings(); }
};
</script>

<template>
  <div class="fixed inset-0 z-100 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="$emit('close')">
    <div class="bg-bg-card rounded-[40px] shadow-2xl w-full max-w-md overflow-hidden flex flex-col">
      <div class="p-8 border-b flex justify-between items-center"><h2 class="text-2xl font-bold">设置</h2><button @click="$emit('close')"><X :size="24" /></button></div>
      <div class="p-10 space-y-8">
        <div class="space-y-3"><label class="text-xs font-bold text-text-sec">截图保存位置</label><div class="flex gap-2"><input type="text" readonly :value="savePath" class="flex-1 bg-bg-input rounded-xl px-4 py-2 text-sm outline-none" /><button @click="selectFolder" class="bg-bg-card border p-2 rounded-xl hover:border-[#007AFF] transition-all"><FolderOpen :size="18" /></button></div></div>
        <div class="space-y-3"><label class="text-xs font-bold text-text-sec">数据库文件</label><div class="flex gap-2"><input type="text" readonly :value="dbPath" class="flex-1 bg-bg-input rounded-xl px-4 py-2 text-sm outline-none" /><div class="flex gap-1"><button @click="selectDBFile" title="打开现有" class="bg-bg-card border p-2 rounded-xl hover:border-[#007AFF] transition-all"><FolderOpen :size="18" /></button><button @click="createDBFile" title="新建" class="bg-bg-card border p-2 rounded-xl hover:border-[#007AFF] transition-all"><Plus :size="18" /></button></div></div></div>
        
        <div class="space-y-2"><div class="flex justify-between"><label class="text-xs font-bold">主题设置</label></div>
           <div class="flex bg-bg-input rounded-xl p-1 gap-1">
              <button @click="theme = 'light'; updateSettings()" class="flex-1 py-1.5 text-xs font-bold rounded-lg transition-all" :class="theme === 'light' ? 'bg-bg-card shadow-sm text-text-main' : 'text-text-sec hover:text-text-main'">浅色</button>
              <button @click="theme = 'dark'; updateSettings()" class="flex-1 py-1.5 text-xs font-bold rounded-lg transition-all" :class="theme === 'dark' ? 'bg-bg-card shadow-sm text-text-main' : 'text-text-sec hover:text-text-main'">深色</button>
              <button @click="theme = 'system'; updateSettings()" class="flex-1 py-1.5 text-xs font-bold rounded-lg transition-all" :class="theme === 'system' ? 'bg-bg-card shadow-sm text-text-main' : 'text-text-sec hover:text-text-main'">跟随系统</button>
           </div>
        </div>

        <div class="space-y-2"><div class="flex justify-between"><label class="text-xs font-bold">截图间隔</label><span class="text-xs font-bold text-[#007AFF]">{{ captureInterval }}s</span></div><input type="range" v-model.number="captureInterval" min="60" max="600" step="10" @change="updateSettings" class="w-full h-1 bg-bg-hover accent-[#007AFF]" /></div>
        <div class="space-y-2"><div class="flex justify-between"><label class="text-xs font-bold">清理策略</label><span class="text-xs font-bold text-[#007AFF]">{{ retainDays }}天</span></div><input type="range" v-model.number="retainDays" min="1" max="180" @change="updateSettings" class="w-full h-1 bg-bg-hover accent-[#007AFF]" /></div>
      </div>
    </div>
  </div>
</template>
