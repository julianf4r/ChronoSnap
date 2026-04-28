<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { X, Plus, Trash2, Check, Clock, Calendar, Edit2 } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";
import { reminders, loadReminders, showToast, currentDate, logicalMinutesToTime, logicalMinutesFromTime, currentLogicalMinute, getLogicDateStr, TIME_OFFSET_MINUTES } from "../../store";
import { Reminder } from "../../types";

const emit = defineEmits(["close"]);

const getInitialMinute = () => {
  const now = new Date();
  const later = new Date(now.getTime() + 10 * 60000);
  const h = later.getHours();
  const m = later.getMinutes();
  return logicalMinutesFromTime(`${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`);
};

const newContent = ref("");
const newMinute = ref(getInitialMinute());
const editingId = ref<number | null>(null);

const isTimePickerOpen = ref(false);
const timePickerRef = ref<HTMLElement | null>(null);

const openTimePicker = async () => {
  isTimePickerOpen.value = !isTimePickerOpen.value;
  if (isTimePickerOpen.value) {
    await nextTick();
    const activeItems = timePickerRef.value?.querySelectorAll('.bg-\\[\\#007AFF\\].text-white');
    activeItems?.forEach(el => el.scrollIntoView({ block: 'center' }));
  }
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  if (isTimePickerOpen.value && timePickerRef.value && !timePickerRef.value.contains(target)) {
    isTimePickerOpen.value = false;
  }
};

onMounted(() => window.addEventListener('mousedown', handleClickOutside));
onUnmounted(() => window.removeEventListener('mousedown', handleClickOutside));

const addReminder = async () => {
  if (!newContent.value.trim()) {
    showToast("请输入提醒内容", "error");
    return;
  }
  const isEditing = editingId.value !== null;
  try {
    await invoke("save_reminder", { 
      reminder: { id: editingId.value || 0, date: currentDate.value, minute: newMinute.value, content: newContent.value, is_completed: false }
    });
    newContent.value = "";
    newMinute.value = getInitialMinute();
    editingId.value = null;
    await loadReminders();
    showToast(isEditing ? "提醒已更新" : "提醒已添加");
  } catch (e) {
    showToast("保存失败: " + e, "error");
  }
};

const editReminder = (r: Reminder) => {
  editingId.value = r.id;
  newContent.value = r.content;
  newMinute.value = r.minute;
};

const cancelEdit = () => {
  editingId.value = null;
  newContent.value = "";
  newMinute.value = getInitialMinute();
};

const toggleStatus = async (r: Reminder) => {
  try {
    await invoke("toggle_reminder", { id: r.id, isCompleted: !r.is_completed });
    await loadReminders();
  } catch (e) {}
};

const deleteItem = async (id: number) => {
  try {
    await invoke("delete_reminder", { id });
    await loadReminders();
    showToast("提醒已删除");
  } catch (e) {}
};

const safeReminders = computed(() => Array.isArray(reminders.value) ? reminders.value : []);

const isPast = computed(() => currentDate.value < getLogicDateStr());
const isToday = computed(() => currentDate.value === getLogicDateStr());

const overdueReminders = computed(() => safeReminders.value.filter(r => 
  !r.is_completed && (isPast.value || (isToday.value && r.minute < currentLogicalMinute.value))
));

const upcomingReminders = computed(() => safeReminders.value.filter(r => 
  !r.is_completed && (!isPast.value && (!isToday.value || r.minute >= currentLogicalMinute.value))
));

const completedReminders = computed(() => safeReminders.value.filter(r => r.is_completed));
</script>

<template>
  <div class="fixed inset-0 z-110 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="emit('close')">
    <div class="bg-bg-card rounded-[40px] shadow-2xl w-full max-w-md overflow-hidden flex flex-col h-[80vh] max-h-175">
      <div class="p-8 border-b border-border-main/50 flex justify-between items-center bg-bg-card/80 backdrop-blur-md sticky top-0 z-10">
        <h2 class="text-2xl font-bold flex items-center gap-3"><Clock :size="24" class="text-[#007AFF]"/> 提醒事项</h2>
        <button @click="emit('close')" class="p-2 hover:bg-bg-input rounded-xl text-text-sec"><X :size="24" /></button>
      </div>

      <div class="p-6 border-b border-border-main/50 bg-bg-input/30 relative">
        <form @submit.prevent="addReminder" class="flex items-center gap-3">
          <div ref="timePickerRef" class="relative">
            <button type="button" @click="openTimePicker" class="bg-bg-card border border-border-main rounded-xl px-4 py-3 text-sm font-bold flex items-center justify-center hover:bg-bg-hover transition-all focus:border-[#007AFF]/50 w-24 outline-none" :class="{ 'ring-2 ring-[#007AFF]/50 border-transparent': editingId }">
              {{ logicalMinutesToTime(newMinute) }}
            </button>
            <div v-if="isTimePickerOpen" class="absolute top-full left-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-4 animate-in fade-in slide-in-from-top-2 flex gap-4 h-64 w-48">
              <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1 text-center">
                <button type="button" v-for="h in 24" :key="h" @click="newMinute = logicalMinutesFromTime(`${String((h-1+3)%24).padStart(2,'0')}:${String((newMinute + TIME_OFFSET_MINUTES) % 60).padStart(2,'0')}`)" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="Math.floor((newMinute + TIME_OFFSET_MINUTES) / 60) % 24 === (h-1+3)%24 ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">
                  {{ String((h-1+3)%24).padStart(2,'0') }}
                </button>
              </div>
              <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1 text-center">
                <button type="button" v-for="m in 60" :key="m" @click="newMinute = Math.floor(newMinute / 60) * 60 + (m-1); isTimePickerOpen = false" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="newMinute % 60 === (m-1) ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">
                  {{ String(m-1).padStart(2,'0') }}
                </button>
              </div>
            </div>
          </div>
          <div class="flex-1 relative">
             <input type="text" v-model="newContent" :placeholder="editingId ? '编辑提醒...' : '添加新提醒...'" class="w-full bg-bg-card border border-border-main rounded-xl px-4 py-3 text-sm font-bold outline-none focus:border-[#007AFF]/50 transition-all pr-10" :class="{ 'ring-2 ring-[#007AFF]/50 border-transparent': editingId }" />
             <button v-if="editingId" type="button" @click="cancelEdit" class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-text-sec hover:bg-bg-input rounded-lg"><X :size="16" /></button>
          </div>
          <button type="submit" class="bg-[#007AFF] text-white p-3 rounded-xl hover:brightness-110 transition-all shadow-md shadow-[#007AFF]/20"><Check v-if="editingId" :size="20" /><Plus v-else :size="20" /></button>
        </form>
      </div>

      <div class="flex-1 overflow-y-auto p-6 space-y-6 no-scrollbar">
        <!-- 已超时未办 -->
        <div v-if="overdueReminders && overdueReminders.length > 0">
          <h3 class="text-xs font-bold text-[#FF3B30] mb-3 uppercase tracking-wider">已超时未办</h3>
          <div class="space-y-2 mb-6">
            <div v-for="r in overdueReminders" :key="r.id" class="flex items-center gap-3 bg-[#FF3B30]/10 p-3 rounded-2xl border border-[#FF3B30]/30 group transition-all hover:bg-[#FF3B30]/20 hover:shadow-sm" :class="{ 'ring-2 ring-[#FF3B30]/50': editingId === r.id }">
              <button @click="toggleStatus(r)" class="w-6 h-6 rounded-full border-2 border-[#FF3B30]/50 flex items-center justify-center text-transparent hover:border-[#FF3B30] transition-all"><Check :size="14" class="opacity-0 group-hover:opacity-50 group-hover:text-[#FF3B30]"/></button>
              <div class="font-bold text-[#FF3B30] text-sm w-12">{{ logicalMinutesToTime(r.minute) }}</div>
              <div class="flex-1 font-medium text-sm text-[#FF3B30] truncate" :class="{ 'opacity-50': editingId === r.id }">{{ r.content }}</div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all">
                <button @click="editReminder(r)" class="p-2 text-text-sec hover:text-[#007AFF] rounded-lg hover:bg-[#007AFF]/10"><Edit2 :size="16" /></button>
                <button @click="deleteItem(r.id)" class="p-2 text-text-sec hover:text-[#FF3B30] rounded-lg hover:bg-[#FF3B30]/20"><Trash2 :size="16" /></button>
              </div>
            </div>
          </div>
        </div>

        <!-- 即将到来 -->
        <div v-if="upcomingReminders && upcomingReminders.length > 0">
          <h3 class="text-xs font-bold text-text-sec mb-3 uppercase tracking-wider">即将到来</h3>
          <div class="space-y-2 mb-6">
            <div v-for="r in upcomingReminders" :key="r.id" class="flex items-center gap-3 bg-bg-input/50 p-3 rounded-2xl border border-border-main/30 group transition-all hover:bg-bg-input hover:shadow-sm" :class="{ 'ring-2 ring-[#007AFF]/50 border-transparent': editingId === r.id }">
              <button @click="toggleStatus(r)" class="w-6 h-6 rounded-full border-2 border-border-main flex items-center justify-center text-transparent hover:border-[#007AFF] transition-all"><Check :size="14" class="opacity-0 group-hover:opacity-30 group-hover:text-[#007AFF]"/></button>
              <div class="font-bold text-[#007AFF] text-sm w-12">{{ logicalMinutesToTime(r.minute) }}</div>
              <div class="flex-1 font-medium text-sm text-text-main truncate" :class="{ 'opacity-50': editingId === r.id }">{{ r.content }}</div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all">
                <button @click="editReminder(r)" class="p-2 text-text-sec hover:text-[#007AFF] rounded-lg hover:bg-[#007AFF]/10"><Edit2 :size="16" /></button>
                <button @click="deleteItem(r.id)" class="p-2 text-text-sec hover:text-[#FF3B30] rounded-lg hover:bg-[#FF3B30]/10"><Trash2 :size="16" /></button>
              </div>
            </div>
          </div>
        </div>

        <!-- 已完成 -->
        <div v-if="completedReminders && completedReminders.length > 0">
          <h3 class="text-xs font-bold text-text-sec mb-3 uppercase tracking-wider">已完成</h3>
          <div class="space-y-2 opacity-60">
            <div v-for="r in completedReminders" :key="r.id" class="flex items-center gap-3 bg-bg-input/30 p-3 rounded-2xl border border-transparent group transition-all" :class="{ 'ring-2 ring-border-main': editingId === r.id }">
              <button @click="toggleStatus(r)" class="w-6 h-6 rounded-full bg-[#007AFF] flex items-center justify-center text-white"><Check :size="14" /></button>
              <div class="font-bold text-text-sec text-sm w-12 line-through">{{ logicalMinutesToTime(r.minute) }}</div>
              <div class="flex-1 font-medium text-sm text-text-sec line-through truncate" :class="{ 'opacity-50': editingId === r.id }">{{ r.content }}</div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all">
                <button @click="editReminder(r)" class="p-2 text-text-sec hover:text-[#007AFF] rounded-lg hover:bg-[#007AFF]/10"><Edit2 :size="16" /></button>
                <button @click="deleteItem(r.id)" class="p-2 text-text-sec hover:text-[#FF3B30] rounded-lg hover:bg-[#FF3B30]/10"><Trash2 :size="16" /></button>
              </div>
            </div>
          </div>
        </div>

        <!-- 无提醒事项 -->
        <div v-if="(!overdueReminders || overdueReminders.length === 0) && (!upcomingReminders || upcomingReminders.length === 0) && (!completedReminders || completedReminders.length === 0)" class="flex flex-col items-center justify-center h-full text-text-sec opacity-40 mt-10">
          <Calendar :size="48" class="mb-4" />
          <p class="font-bold">今天没有提醒事项</p>
        </div>
      </div>
    </div>
  </div>
</template>