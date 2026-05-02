<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, Trash2, ChevronRight, ChevronDown } from "lucide-vue-next";
import { mainTags, getSubTags, getTagName, showToast, loadTags } from "../../store";

defineEmits(['close']);

const expandedMainTags = ref<number[]>([]);
const toggleMainTag = (id: number) => {
  const index = expandedMainTags.value.indexOf(id);
  if (index > -1) expandedMainTags.value.splice(index, 1);
  else expandedMainTags.value.push(id);
};

const isTagSelectOpen = ref(false);
const newTagName = ref("");
const newTagParent = ref<number | null>(null);
const newTagColor = ref("#007AFF");

const handleAddTag = async () => {
  try {
    await invoke("add_tag", { name: newTagName.value, parentId: newTagParent.value, color: newTagColor.value });
    await loadTags();
    showToast("标签添加成功");
    newTagName.value = ""; newTagParent.value = null; newTagColor.value = "#007AFF";
  } catch (e) {
    showToast("添加失败: " + e, "error");
  }
};

const handleDeleteTag = async (id: number) => {
  try {
    await invoke("delete_tag", { id });
    await loadTags();
    showToast("标签已删除");
  } catch (e: any) {
    if (e.toString().includes("FOREIGN KEY") || e.toString().includes("正在被事件使用")) {
      showToast("该标签正在被使用，无法删除", "error");
    } else {
      showToast("删除失败: " + e, "error");
    }
  }
};
</script>

<template>
  <div class="fixed inset-0 z-100 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="$emit('close')">
    <div class="bg-bg-card rounded-[40px] shadow-2xl w-full max-w-2xl h-[80vh] overflow-hidden flex flex-col animate-in fade-in zoom-in duration-200">
      <div class="p-8 border-b flex justify-between items-center"><h2 class="text-2xl font-bold">标签管理</h2><button @click="$emit('close')"><X :size="24" /></button></div>
      <div class="flex-1 overflow-hidden p-10 flex gap-10">
        <div class="flex-1 overflow-y-auto space-y-2 pr-4 no-scrollbar">
          <div v-for="mt in mainTags" :key="mt.id" class="space-y-2">
            <div @click="toggleMainTag(mt.id)" class="flex items-center justify-between group p-2 hover:bg-bg-input rounded-xl cursor-pointer">
              <div class="flex items-center gap-3">
                <ChevronRight :size="14" class="text-text-sec transition-transform" :class="{ 'rotate-90': expandedMainTags.includes(mt.id) }" />
                <div class="w-4 h-4 rounded-full" :style="{ backgroundColor: mt.color }"></div>
                <span class="font-bold">{{ mt.name }}</span>
              </div>
              <button @click.stop="handleDeleteTag(mt.id)" class="text-[#FF3B30] opacity-0 group-hover:opacity-100"><Trash2 :size="16" /></button>
            </div>
            <div v-if="expandedMainTags.includes(mt.id)" class="ml-7 space-y-1 animate-in fade-in slide-in-from-top-1 duration-200">
              <div v-for="st in getSubTags(mt.id)" :key="st.id" class="flex items-center justify-between group p-1.5 hover:bg-bg-input rounded-lg">
                <span class="text-sm">{{ st.name }}</span>
                <button @click="handleDeleteTag(st.id)" class="text-[#FF3B30] opacity-0 group-hover:opacity-100"><Trash2 :size="14" /></button>
              </div>
            </div>
          </div>
        </div>
        <div class="w-64 bg-bg-input p-6 rounded-3xl space-y-4 h-fit">
          <h3 class="font-bold text-xs text-text-sec uppercase">添加标签</h3>
          <div class="space-y-1">
            <label class="text-[10px] font-bold text-text-sec ml-1">标签名称</label>
            <input v-model="newTagName" placeholder="输入名称..." class="w-full bg-bg-card rounded-xl px-4 py-2.5 text-sm outline-none border border-transparent focus:border-[#007AFF] transition-all" />
          </div>
          <div class="space-y-1 relative">
            <label class="text-[10px] font-bold text-text-sec ml-1">父级标签</label>
            <button @click="isTagSelectOpen = !isTagSelectOpen" class="w-full bg-bg-card rounded-xl px-4 py-2.5 text-sm text-left flex justify-between items-center border border-transparent focus:border-[#007AFF] transition-all">
              <span>{{ getTagName(newTagParent) }}</span>
              <ChevronDown :size="14" class="text-text-sec transition-transform" :class="{ 'rotate-180': isTagSelectOpen }" />
            </button>
            <div v-if="isTagSelectOpen" class="absolute top-full left-0 right-0 mt-2 bg-bg-card rounded-2xl shadow-xl border border-border-main z-60 overflow-hidden py-2 animate-in fade-in zoom-in-95 duration-200">
              <div @click="newTagParent = null; isTagSelectOpen = false" class="px-4 py-2 text-sm hover:bg-bg-input cursor-pointer" :class="{ 'text-[#007AFF] font-bold': newTagParent === null }">-- 无 --</div>
              <div v-for="t in mainTags" :key="t.id" @click="newTagParent = t.id; isTagSelectOpen = false" class="px-4 py-2 text-sm hover:bg-bg-input cursor-pointer" :class="{ 'text-[#007AFF] font-bold': newTagParent === t.id }">{{ t.name }}</div>
            </div>
          </div>
          <div v-if="newTagParent === null" class="space-y-1">
            <label class="text-[10px] font-bold text-text-sec ml-1">主题颜色</label>
            <div class="flex flex-wrap gap-2"><button v-for="c in ['#007AFF', '#34C759', '#FF9500', '#FF3B30', '#AF52DE', '#5856D6']" :key="c" @click="newTagColor = c" class="w-5 h-5 rounded-full border-2 transition-all" :style="{ backgroundColor: c, borderColor: newTagColor === c ? '#1D1D1F' : 'transparent' }"></button></div>
          </div>
          <button @click="handleAddTag" :disabled="!newTagName" class="w-full bg-[#007AFF] text-white py-3 rounded-xl font-bold shadow-lg shadow-[#007AFF]/20 active:scale-95 transition-all">创建</button>
        </div>
      </div>
    </div>
  </div>
</template>
