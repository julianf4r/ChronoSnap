<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick, watch } from "vue";
import { load as loadStore } from "@tauri-apps/plugin-store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { 
  RefreshCw, Calendar, ChevronLeft, ChevronRight, Play, Pause, SquarePlus, Tag as TagIcon, Download, Settings, Maximize2, X, Image as ImageIcon, BarChart2, Trash2, Bell 
} from "lucide-vue-next";

import { 
  isSetupComplete, savePath, dbPath, isPaused, currentDate, currentLogicalMinute, getLogicDateStr, timelineImages, selectedImage, lockedImage, 
  previewSrc, isFullscreen, dayEvents, reminders, toast, showToast, timelineZoom, theme, viewMode,
  retainDays, captureInterval,
  TIME_OFFSET_MINUTES, TOTAL_MINUTES, getTagColor, getTagName, mainTags, getSubTags,
  logicalMinutesToTime, logicalMinutesFromTime, formatDuration,
  loadTags, loadEvents, loadReminders, toISODate, refreshBadgeCount,
  calendarStatus, loadCalendarStatus, currentCalendarMonthStr
} from "./store";
import { DBEvent, TimelineItem } from "./types";

// --- Components ---
import Setup from "./components/Setup.vue";
import Preview from "./components/views/Preview.vue";
import Dashboard from "./components/views/Dashboard.vue";
import TagManager from "./components/modals/TagManager.vue";
import ExportModal from "./components/modals/ExportModal.vue";
import SettingsModal from "./components/modals/SettingsModal.vue";
import ReminderManager from "./components/modals/ReminderManager.vue";

// --- Local State (Timeline UI) ---
const isSettingsOpen = ref(false);
const isTagManagerOpen = ref(false);
const isExportModalOpen = ref(false);
const isEventModalOpen = ref(false);
const isCalendarOpen = ref(false);
const isReminderManagerOpen = ref(false);
const isMouseOverTimeline = ref(false); // Track mouse state to prevent race conditions

const calendarRef = ref<HTMLElement | null>(null);
const calendarMonth = ref(new Date());
const timelineRef = ref<HTMLElement | null>(null);

const isDragging = ref(false);
const dragStartMin = ref<number | null>(null);
const dragEndMin = ref<number | null>(null);
const hoveredTime = ref<string | null>(null);
const hoveredEventDetails = ref<{ event: DBEvent; x: number; y: number } | null>(null);

const editingEvent = ref<DBEvent>({ id: 0, date: "", start_minute: 0, end_minute: 0, main_tag_id: 0, sub_tag_id: null, content: "" });

// --- Time Pickers State ---
const startTimePickerRef = ref<HTMLElement | null>(null);
const endTimePickerRef = ref<HTMLElement | null>(null);
const isStartTimeOpen = ref(false);
const isEndTimeOpen = ref(false);

const openStartTimePicker = async () => {
  isStartTimeOpen.value = !isStartTimeOpen.value;
  isEndTimeOpen.value = false;
  if (isStartTimeOpen.value) {
    await nextTick();
    const activeItems = document.querySelectorAll('.z-120 .bg-\\[\\#007AFF\\].text-white');
    activeItems.forEach(el => el.scrollIntoView({ block: 'center' }));
  }
};

const openEndTimePicker = async () => {
  isEndTimeOpen.value = !isEndTimeOpen.value;
  isStartTimeOpen.value = false;
  if (isEndTimeOpen.value) {
    await nextTick();
    const activeItems = document.querySelectorAll('.z-120 .bg-\\[\\#007AFF\\].text-white');
    activeItems.forEach(el => el.scrollIntoView({ block: 'center' }));
  }
};

// --- Reminders Check Logic ---
let lastCheckedMinute = -1;
const checkReminders = async (currentMin: number) => {
  if (lastCheckedMinute === currentMin) return;
  lastCheckedMinute = currentMin;

  const dueReminders = reminders.value.filter(r => !r.is_completed && r.minute === currentMin);
  if (dueReminders.length > 0) {
    let hasPermission = await isPermissionGranted();
    if (!hasPermission) {
      const permission = await requestPermission();
      hasPermission = permission === 'granted';
    }

    for (const r of dueReminders) {
      if (hasPermission) {
        sendNotification({ title: '', body: r.content });
      }
    }
  }
};

// --- Logic ---
const updateCurrentMinute = () => {
  const now = new Date();
  const logicalDateStr = new Date(now.getTime() - TIME_OFFSET_MINUTES * 60000).toLocaleDateString('sv');
  if (logicalDateStr === currentDate.value) {
    const m = now.getHours() * 60 + now.getMinutes();
    const min = (m < TIME_OFFSET_MINUTES ? m + 1440 : m) - TIME_OFFSET_MINUTES;
    currentLogicalMinute.value = min;
    checkReminders(min);
  } else {
    currentLogicalMinute.value = -1;
  }
  // 无论是否看今天，每分钟都尝试刷新全局徽章和日历状态点
  refreshBadgeCount();
};

let currentMinuteTimeout: number | null = null;
let currentMinuteInterval: number | null = null;
let captureUnlisten: any = null;
let timelineZoomSaveTimeout: number | null = null;
const previewCache = new Map<string, string>();
const MAX_PREVIEW_CACHE_SIZE = 30;

const startMinuteTimer = () => {
  updateCurrentMinute();
  const now = new Date();
  const msUntilNextMinute = 60000 - (now.getSeconds() * 1000 + now.getMilliseconds());
  currentMinuteTimeout = window.setTimeout(() => {
    updateCurrentMinute();
    currentMinuteInterval = window.setInterval(updateCurrentMinute, 60000);
  }, msUntilNextMinute);
};

watch(calendarMonth, (newMonth) => {
  const y = newMonth.getFullYear();
  const m = String(newMonth.getMonth() + 1).padStart(2, '0');
  currentCalendarMonthStr.value = `${y}-${m}`;
  loadCalendarStatus(currentCalendarMonthStr.value);
}, { immediate: true });

watch(currentDate, () => {
  updateCurrentMinute();
});

onMounted(async () => {
  window.addEventListener('mousedown', handleClickOutside);
  startMinuteTimer();
  
  const store = await loadStore("config.json");
  const path = await store.get<string>("savePath");
  const dPath = await store.get<string>("dbPath");
  
  if (path && dPath) {
    savePath.value = path;
    dbPath.value = dPath;
    isSetupComplete.value = true;
    await invoke("update_db_path", { path: dbPath.value });
    
    const rDays = await store.get<number>("retainDays");
    if (rDays) retainDays.value = rDays;
    const cInterval = await store.get<number>("captureInterval");
    if (cInterval) captureInterval.value = Math.max(cInterval, 60);
    const tZoom = await store.get<number>("timelineZoom");
    if (tZoom) timelineZoom.value = tZoom;
    const tTheme = await store.get<string>("theme");
    if (tTheme) theme.value = tTheme;

    applyTheme(theme.value);
    await invoke("update_interval", { seconds: captureInterval.value });
    isPaused.value = await invoke<boolean>("get_pause_state");
    
    await loadTimeline(true); await loadTags(); await loadEvents(); await loadReminders();
  }

  await listen<boolean>("pause-state-changed", (event) => { isPaused.value = event.payload; });
  captureUnlisten = await listen("refresh-timeline", () => { loadTimeline(); });
});

onUnmounted(() => { 
  window.removeEventListener('mousedown', handleClickOutside);
  if (currentMinuteTimeout) window.clearTimeout(currentMinuteTimeout);
  if (currentMinuteInterval) window.clearInterval(currentMinuteInterval);
  if (timelineZoomSaveTimeout) window.clearTimeout(timelineZoomSaveTimeout);
  if (captureUnlisten) captureUnlisten(); 
});

const handleSetupComplete = async () => {
  applyTheme(theme.value);
  updateCurrentMinute();
  await loadTags();
  await loadEvents();
  await loadReminders();
  await loadTimeline(true);
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  if (isCalendarOpen.value && calendarRef.value && !calendarRef.value.contains(target)) isCalendarOpen.value = false;
  if (isStartTimeOpen.value && startTimePickerRef.value && !startTimePickerRef.value.contains(target)) isStartTimeOpen.value = false;
  if (isEndTimeOpen.value && endTimePickerRef.value && !endTimePickerRef.value.contains(target)) isEndTimeOpen.value = false;
};

const applyTheme = (val: string) => {
  const isDark = val === 'dark' || (val === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.classList.toggle('dark', isDark);
};

const loadTimeline = async (autoScroll = false) => { 
  if (savePath.value) {
    const rawImages = await invoke<TimelineItem[]>("get_timeline", { date: currentDate.value, baseDir: savePath.value }); 
    // Pre-calculate logical minutes to prevent high-cost recalculation on every mouse move
    timelineImages.value = rawImages.map(img => ({
      ...img,
      logical_minute: timeToLogicalMinutes(img.time, img.isNextDay)
    }));
    
    if (autoScroll && timelineRef.value) {
      await nextTick();
      const now = new Date();
      const currentMin = now.getHours() * 60 + now.getMinutes();
      const logicalMin = (currentMin < TIME_OFFSET_MINUTES ? currentMin + 1440 : currentMin) - TIME_OFFSET_MINUTES;
      timelineRef.value.scrollTop = Math.max(0, logicalMin * timelineZoom.value - timelineRef.value.clientHeight / 2);
    }
  }
};

const updatePreview = async (img: TimelineItem | null) => {
  if (!img || (selectedImage.value?.path === img.path && previewSrc.value)) return;
  selectedImage.value = img;
  const cachedSrc = previewCache.get(img.path);
  if (cachedSrc) {
    previewSrc.value = cachedSrc;
    return;
  }

  const b64 = await invoke<string>("get_image_base64", { path: img.path });
  const src = `data:image/jpeg;base64,${b64}`;
  previewSrc.value = src;
  previewCache.set(img.path, src);
  if (previewCache.size > MAX_PREVIEW_CACHE_SIZE) {
    const oldestKey = previewCache.keys().next().value;
    if (oldestKey) previewCache.delete(oldestKey);
  }
};

// --- Timeline Handlers ---
const handleTimelineMouseDown = (e: MouseEvent) => {
  if (!timelineRef.value) return;
  const min = Math.floor((e.clientY - timelineRef.value.getBoundingClientRect().top + timelineRef.value.scrollTop) / timelineZoom.value);
  isDragging.value = true; dragStartMin.value = min; dragEndMin.value = min;
};

const timeToLogicalMinutes = (timeStr: string, isNextDay = false) => {
  const [h, m, s = 0] = timeStr.split(":").map(Number);
  let t = h * 60 + m + (s / 60); if (isNextDay) t += 1440; return t - TIME_OFFSET_MINUTES;
};

const findClosestImage = (minute: number) => {
  const images = timelineImages.value;
  if (images.length === 0) return null;

  let low = 0;
  let high = images.length - 1;
  while (low <= high) {
    const mid = Math.floor((low + high) / 2);
    const midMinute = images[mid].logical_minute ?? 0;
    if (midMinute < minute) low = mid + 1;
    else high = mid - 1;
  }

  const next = images[low];
  const prev = images[low - 1];
  if (!prev) return next;
  if (!next) return prev;
  return Math.abs((prev.logical_minute ?? 0) - minute) <= Math.abs((next.logical_minute ?? 0) - minute) ? prev : next;
};

// Request Animation Frame lock for mouse move
let isMouseMovePending = false;
const handleTimelineMouseMove = (e: MouseEvent) => {
  if (!timelineRef.value) return;
  isMouseOverTimeline.value = true;
  
  if (isMouseMovePending) return;
  isMouseMovePending = true;

  const clientY = e.clientY; // Capture coordinate outside RAF

  requestAnimationFrame(() => {
    isMouseMovePending = false;
    
    // Safety check: If mouse already left before this frame executed, DO NOT update preview
    if (!isMouseOverTimeline.value || !timelineRef.value) return;

    const rect = timelineRef.value.getBoundingClientRect();
    const min = Math.floor((clientY - rect.top + timelineRef.value.scrollTop) / timelineZoom.value);
    
    if (min >= 0 && min < 1440) {
      hoveredTime.value = logicalMinutesToTime(min);
      if (isDragging.value) dragEndMin.value = min;
      if (viewMode.value === 'dashboard') return;
      
      const closest = findClosestImage(min);
      if (closest) updatePreview(closest);
    }
  });
};

const handleTimelineMouseLeave = () => {
  isMouseOverTimeline.value = false; // Immediately flag as left
  hoveredTime.value = null;
  if (viewMode.value === 'dashboard') return;
  if (!isDragging.value && lockedImage.value) {
    updatePreview(lockedImage.value);
  }
};

const handleTimelineMouseUp = () => {
  if (isDragging.value) {
    isDragging.value = false;
    const start = Math.min(dragStartMin.value!, dragEndMin.value!);
    const end = Math.max(dragStartMin.value!, dragEndMin.value!);
    if (end - start >= 1) {
      editingEvent.value = { id: 0, date: currentDate.value, start_minute: start, end_minute: end, main_tag_id: mainTags.value[0]?.id || 0, sub_tag_id: null, content: "" };
      isEventModalOpen.value = true;
    } else if (timelineImages.value.length) {
        const closest = findClosestImage(start);
        if (closest) { lockedImage.value = closest; viewMode.value = 'preview'; updatePreview(closest); }
    }
  }
};

const handleTimelineWheel = (e: WheelEvent) => {
  if (e.ctrlKey) {
    e.preventDefault(); if (!timelineRef.value) return;
    const rect = timelineRef.value.getBoundingClientRect();
    const my = e.clientY - rect.top; const cy = my + timelineRef.value.scrollTop;
    const oh = TOTAL_MINUTES * timelineZoom.value;
    timelineZoom.value = Math.min(Math.max(0.5, timelineZoom.value + (e.deltaY > 0 ? -0.2 : 0.2)), 15);
    timelineRef.value.scrollTop = (cy / oh) * (TOTAL_MINUTES * timelineZoom.value) - my;
    persistTimelineZoom();
  }
};

const persistTimelineZoom = () => {
  if (timelineZoomSaveTimeout) window.clearTimeout(timelineZoomSaveTimeout);
  timelineZoomSaveTimeout = window.setTimeout(async () => {
    const store = await loadStore("config.json");
    await store.set("timelineZoom", timelineZoom.value);
    await store.save();
  }, 400);
};

const selectCalendarDate = (date: Date) => {
  currentDate.value = toISODate(date);
  isCalendarOpen.value = false;
  selectedImage.value = null;
  lockedImage.value = null;
  previewSrc.value = "";
  hoveredTime.value = null;
  hoveredEventDetails.value = null;
  dayEvents.value = []
  timelineImages.value = [];
  updateCurrentMinute();
  loadTimeline(true);
  loadEvents();
  loadReminders();
};

const calendarDays = computed(() => {
  const y = calendarMonth.value.getFullYear(); const m = calendarMonth.value.getMonth();
  const firstDay = new Date(y, m, 1).getDay(); const daysInMonth = new Date(y, m + 1, 0).getDate();
  const days = []; const padding = (firstDay + 6) % 7;
  for (let i = 0; i < padding; i++) days.push(null);
  for (let i = 1; i <= daysInMonth; i++) days.push(new Date(y, m, i));
  return days;
});

const saveEvent = async () => {
  if (editingEvent.value.end_minute <= editingEvent.value.start_minute) {
    showToast("结束时间必须晚于开始时间", "error");
    return;
  }
  try {
    await invoke("save_event", { event: editingEvent.value });
    isEventModalOpen.value = false; await loadEvents(); showToast("事件已保存");
  } catch (e) { showToast("保存失败: " + e, "error"); }
};

const deleteEvent = async (id: number) => {
  try { await invoke("delete_event", { id }); await loadEvents(); showToast("事件已删除"); } catch (e) { showToast("删除失败: " + e, "error"); }
};

const handleQuickAddEvent = () => {
  let start = dayEvents.value.length ? Math.max(...dayEvents.value.map(e => e.end_minute)) : 0;
  let end = currentLogicalMinute.value >= 0 ? currentLogicalMinute.value : Math.min(start + 30, 1439);
  editingEvent.value = { id: 0, date: currentDate.value, start_minute: Math.min(start, end), end_minute: end, main_tag_id: mainTags.value[0]?.id || 0, sub_tag_id: null, content: "" };
  isEventModalOpen.value = true;
};

const togglePause = async () => {
  isPaused.value = await invoke<boolean>("toggle_pause");
};
</script>

<template>
  <div class="h-screen w-screen flex flex-col overflow-hidden text-text-main bg-bg-main">
    <Setup v-if="!isSetupComplete" @complete="handleSetupComplete" />
    <div v-else class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <div class="w-80 bg-bg-sidebar border-r border-border-main flex flex-col select-none relative">
        <div class="p-6 bg-bg-sidebar/80 backdrop-blur-md sticky top-0 z-100 border-b border-border-main/50">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-lg font-bold">瞬影 - 时间记录</h2>
            <div class="flex gap-1">
              <button @click="isReminderManagerOpen = true" class="relative p-2 hover:bg-bg-card rounded-xl text-text-sec">
                <Bell :size="18" />
                <span v-if="reminders.some(r => !r.is_completed && (currentDate < getLogicDateStr() || (currentDate === getLogicDateStr() && r.minute < currentLogicalMinute)))" class="absolute top-1.5 right-1.5 w-2 h-2 bg-[#FF3B30] rounded-full border border-bg-sidebar"></span>
                <span v-else-if="reminders.some(r => !r.is_completed)" class="absolute top-1.5 right-1.5 w-2 h-2 bg-[#007AFF] rounded-full border border-bg-sidebar"></span>
              </button>
              <button @click="loadTimeline(true); loadEvents(); loadReminders()" class="p-2 hover:bg-bg-card rounded-xl text-text-sec"><RefreshCw :size="18" /></button>
            </div>
          </div>
          <div ref="calendarRef" class="relative">
             <button @click="isCalendarOpen = !isCalendarOpen" class="w-full bg-bg-card border border-border-main rounded-xl pl-11 pr-4 py-2.5 text-sm font-bold text-left flex items-center hover:bg-bg-input">
                <Calendar :size="16" class="absolute left-4 text-text-sec" />{{ currentDate }}
             </button>
             <div v-if="isCalendarOpen" class="absolute top-full left-0 right-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-100 p-5 animate-in fade-in zoom-in-95 duration-200">
                <div class="flex items-center justify-between mb-4">
                  <button @click="calendarMonth = new Date(calendarMonth.getFullYear(), calendarMonth.getMonth()-1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronLeft :size="18"/></button>
                  <span class="text-sm font-black">{{ calendarMonth.getFullYear() }}年 {{ calendarMonth.getMonth()+1 }}月</span>
                  <button @click="calendarMonth = new Date(calendarMonth.getFullYear(), calendarMonth.getMonth()+1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronRight :size="18"/></button>
                </div>
                <div class="grid grid-cols-7 gap-1 text-center mb-2"><div v-for="d in ['一','二','三','四','五','六','日']" :key="d" class="text-[10px] font-bold text-text-sec">{{d}}</div></div>
                <div class="grid grid-cols-7 gap-1">
                  <div v-for="(date, i) in calendarDays" :key="i" class="aspect-square flex flex-col items-center justify-center relative">
                    <button v-if="date" @click="selectCalendarDate(date)" 
                      class="w-8 h-8 rounded-full text-xs font-medium transition-all flex items-center justify-center relative" 
                      :class="[
                        date.toLocaleDateString('sv') === getLogicDateStr() 
                          ? 'bg-[#007AFF] text-white font-black shadow-[0_4px_12px_rgba(0,122,255,0.4)] ' + (date.toLocaleDateString('sv') === currentDate ? 'ring-2 ring-offset-2 ring-[#007AFF]' : '')
                          : (date.toLocaleDateString('sv') === currentDate 
                              ? 'bg-bg-input text-[#007AFF] font-black ring-1 ring-border-main shadow-sm' 
                              : 'hover:bg-bg-input text-main')
                      ]">
                      {{ date.getDate() }}
                    </button>
                    <!-- Status Dots -->
                    <div v-if="date && calendarStatus[date.toLocaleDateString('sv')]" class="absolute -bottom-1.5 flex gap-0.5">
                      <div v-if="calendarStatus[date.toLocaleDateString('sv')].has_overdue" class="w-1.5 h-1.5 rounded-full bg-[#FF3B30] border-[1.5px] border-bg-card shadow-sm"></div>
                      <div v-if="calendarStatus[date.toLocaleDateString('sv')].has_upcoming" class="w-1.5 h-1.5 rounded-full bg-[#007AFF] border-[1.5px] border-bg-card shadow-sm"></div>
                    </div>
                  </div>
                </div>
             </div>
          </div>
        </div>

        <div ref="timelineRef" class="flex-1 overflow-y-auto no-scrollbar hover:cursor-crosshair relative" @mousedown="handleTimelineMouseDown" @mousemove="handleTimelineMouseMove" @mouseup="handleTimelineMouseUp" @mouseleave="handleTimelineMouseLeave" @wheel="handleTimelineWheel">
          <div :style="{ height: TOTAL_MINUTES * timelineZoom + 'px' }" class="relative ml-14 mr-4">
            <div v-for="h in 24" :key="h" class="absolute left-0 w-full border-t border-border-main/60" :style="{ top: (h-1) * 60 * timelineZoom + 'px' }">
              <span v-if="h > 1" class="absolute -left-11 -top-2.5 text-[10px] font-bold text-text-sec">{{ String((h - 1 + 3) % 24).padStart(2, '0') }}:00</span>
            </div>
            
            <!-- Reminder Markers -->
            <div v-for="r in reminders" :key="'rem-'+r.id" class="absolute left-0 w-12 h-0.5 rounded-r-full z-40 transition-all" 
                 :class="r.is_completed ? 'bg-text-sec/30' : 
                         (currentDate < getLogicDateStr() || (currentDate === getLogicDateStr() && r.minute < currentLogicalMinute) 
                          ? 'bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.6)]' 
                          : 'bg-[#007AFF] shadow-[0_0_8px_rgba(0,122,255,0.6)]')" 
                 :style="{ top: r.minute * timelineZoom + 'px' }"></div>
            
            <div v-for="ev in dayEvents" :key="ev.id" class="absolute left-0 w-[45%] opacity-80 border-l-4 cursor-pointer hover:opacity-100 hover:border-l-8 hover:z-50 hover:brightness-110" :style="{ top: ev.start_minute * timelineZoom + 'px', height: (ev.end_minute - ev.start_minute) * timelineZoom + 'px', backgroundColor: getTagColor(ev.main_tag_id) + '50', borderColor: getTagColor(ev.main_tag_id) }" @click.stop="editingEvent = { ...ev }; isEventModalOpen = true" @mousedown.stop @mouseenter="hoveredEventDetails = { event: ev, x: $event.clientX, y: $event.clientY }" @mousemove="hoveredEventDetails ? (hoveredEventDetails.x = $event.clientX, hoveredEventDetails.y = $event.clientY) : null" @mouseleave="hoveredEventDetails = null"></div>
            <div v-for="img in timelineImages" :key="img.path" class="absolute left-[50%] right-2 h-0.5 bg-[#007AFF]/20 rounded-full" :class="[selectedImage?.path === img.path ? 'bg-[#007AFF]/60 h-1 z-10' : '', lockedImage?.path === img.path ? 'bg-[#007AFF] h-1.5 ring-2 ring-[#007AFF]/20 z-20' : '']" :style="{ top: (img.logical_minute ?? 0) * timelineZoom + 'px' }"></div>
            <div v-if="isDragging && dragStartMin !== null && dragEndMin !== null" class="absolute left-0 w-full bg-[#007AFF]/10 border-y-2 border-[#007AFF] pointer-events-none z-30" :style="{ top: Math.min(dragStartMin, dragEndMin) * timelineZoom + 'px', height: Math.abs(dragEndMin - dragStartMin) * timelineZoom + 'px' }"></div>
            <div v-if="hoveredTime" class="absolute left-0 right-0 border-t-2 border-[#007AFF] z-40 pointer-events-none" :style="{ top: timeToLogicalMinutes(hoveredTime, hoveredTime < '03:00') * timelineZoom + 'px' }"><div class="absolute -left-12 -top-3 bg-[#007AFF] text-white text-[9px] px-1 py-0.5 rounded font-bold">{{ hoveredTime }}</div></div>
            <div v-if="currentLogicalMinute >= 0" class="absolute left-0 right-0 border-t-2 border-[#FF3B30] z-30 pointer-events-none" :style="{ top: currentLogicalMinute * timelineZoom + 'px' }"><div class="absolute -left-12 -top-2.5 bg-[#FF3B30] text-white text-[9px] px-1 py-0.5 rounded font-bold shadow-[0_0_8px_rgba(255,59,48,0.5)]">{{ logicalMinutesToTime(currentLogicalMinute) }}</div></div>
          </div>
        </div>

        <div class="p-4 border-t border-border-main bg-bg-card/50 flex justify-around">
          <button @click="togglePause" class="p-3 rounded-2xl" :class="isPaused ? 'text-[#FF3B30]' : 'text-text-sec'"><Play v-if="isPaused" :size="22" /><Pause v-else :size="22" /></button>
          <button @click="handleQuickAddEvent" class="p-3 rounded-2xl text-text-sec"><SquarePlus :size="22" /></button>
          <button @click="isTagManagerOpen = true" class="p-3 rounded-2xl text-text-sec"><TagIcon :size="22" /></button>
          <button @click="isExportModalOpen = true" class="p-3 rounded-2xl text-text-sec"><Download :size="22" /></button>
          <button @click="isSettingsOpen = true" class="p-3 rounded-2xl text-text-sec"><Settings :size="22" /></button>
        </div>
      </div>

      <!-- Main Panel -->
      <div class="flex-1 bg-bg-main flex flex-col relative overflow-hidden">
        <div class="p-6 flex items-center justify-between border-b bg-bg-card/80 backdrop-blur-md z-10">
          <div class="flex items-center gap-3">
            <span class="text-lg font-bold">{{ viewMode === 'preview' ? (selectedImage ? selectedImage.time : '预览') : '时间分析' }}</span>
            <span v-if="viewMode === 'preview' && selectedImage" class="text-xs font-bold px-2 py-0.5 rounded-md" :class="lockedImage?.path === selectedImage.path ? 'bg-[#007AFF] text-white' : 'bg-bg-input text-text-sec'">{{ lockedImage?.path === selectedImage.path ? '已定格' : '预览中' }}</span>
          </div>
          <div class="flex items-center gap-4">
            <button v-if="viewMode === 'preview' && selectedImage" @click="isFullscreen = true" class="p-2.5 text-text-sec hover:text-text-main"><Maximize2 :size="20" /></button>
            <div class="flex bg-bg-input rounded-xl p-1 gap-1 border border-border-main/50 shadow-inner">
               <button @click="viewMode = 'preview'" class="px-3 py-1.5 text-[11px] font-bold rounded-lg transition-all flex items-center gap-1.5" :class="viewMode === 'preview' ? 'bg-bg-card shadow-md text-text-main' : 'text-text-sec hover:text-text-main'"><ImageIcon :size="14"/> 预览</button>
               <button @click="viewMode = 'dashboard'" class="px-3 py-1.5 text-[11px] font-bold rounded-lg transition-all flex items-center gap-1.5" :class="viewMode === 'dashboard' ? 'bg-bg-card shadow-md text-text-main' : 'text-text-sec hover:text-text-main'"><BarChart2 :size="14"/> 统计</button>
            </div>
          </div>
        </div>

        <div class="flex-1 relative overflow-hidden">
          <Preview v-show="viewMode === 'preview'" />
          <Dashboard v-if="viewMode === 'dashboard'" />
        </div>
      </div>
    </div>

    <!-- Modals -->
    <TagManager v-if="isTagManagerOpen" @close="isTagManagerOpen = false" />
    <ExportModal v-if="isExportModalOpen" @close="isExportModalOpen = false" />
    <SettingsModal v-if="isSettingsOpen" @close="isSettingsOpen = false" />
    <ReminderManager v-if="isReminderManagerOpen" @close="isReminderManagerOpen = false" />
    
    <!-- Event Modal -->
    <div v-if="isEventModalOpen" class="fixed inset-0 z-110 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="isEventModalOpen = false">
      <div class="bg-bg-card rounded-[40px] shadow-2xl w-full max-w-lg overflow-hidden flex flex-col">
        <div class="p-8 border-b flex justify-between items-center"><h2 class="text-2xl font-bold">{{ editingEvent.id ? '编辑事件' : '新增事件' }}</h2><button @click="isEventModalOpen = false"><X :size="24" /></button></div>
        <div class="p-10 space-y-8">
          <div class="flex gap-3 items-end">
            <div ref="startTimePickerRef" class="flex-1 relative">
              <label class="text-[10px] font-bold text-text-sec block mb-1">开始时间</label>
              <button @click="openStartTimePicker" class="w-full h-12 bg-bg-input rounded-xl px-4 flex items-center justify-center gap-2 hover:bg-bg-hover transition-all border border-transparent focus:border-[#007AFF]/30"><span class="text-sm font-bold leading-none">{{ logicalMinutesToTime(editingEvent.start_minute) }}</span></button>
              <div v-if="isStartTimeOpen" class="absolute top-full left-0 right-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-4 animate-in fade-in slide-in-from-top-2 flex gap-4 h-64">
                 <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1">
                    <button v-for="h in 24" :key="h" @click="editingEvent.start_minute = logicalMinutesFromTime(`${String((h-1+3)%24).padStart(2,'0')}:${String(editingEvent.start_minute % 60).padStart(2,'0')}`)" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="Math.floor((editingEvent.start_minute + TIME_OFFSET_MINUTES) / 60) % 24 === (h-1+3)%24 ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ String((h-1+3)%24).padStart(2,'0') }}</button>
                 </div>
                 <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1">
                    <button v-for="m in 60" :key="m" @click="editingEvent.start_minute = Math.floor(editingEvent.start_minute / 60) * 60 + (m-1); isStartTimeOpen = false" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="editingEvent.start_minute % 60 === (m-1) ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ String(m-1).padStart(2,'0') }}</button>
                 </div>
              </div>
            </div>
            <div class="h-12 flex items-center justify-center"><span class="text-[11px] font-bold text-text-sec bg-bg-input px-3 py-1.5 rounded-xl border border-border-main/50">{{ formatDuration(editingEvent.start_minute, editingEvent.end_minute) }}</span></div>
            <div ref="endTimePickerRef" class="flex-1 relative">
              <label class="text-[10px] font-bold text-text-sec block mb-1">结束时间</label>
              <button @click="openEndTimePicker" class="w-full h-12 bg-bg-input rounded-xl px-4 flex items-center justify-center gap-2 hover:bg-bg-hover transition-all border border-transparent focus:border-[#007AFF]/30"><span class="text-sm font-bold leading-none">{{ logicalMinutesToTime(editingEvent.end_minute) }}</span></button>
              <div v-if="isEndTimeOpen" class="absolute top-full left-0 right-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-4 animate-in fade-in slide-in-from-top-2 flex gap-4 h-64">
                 <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1 text-center">
                    <button v-for="h in 24" :key="h" @click="editingEvent.end_minute = logicalMinutesFromTime(`${String((h-1+3)%24).padStart(2,'0')}:${String(editingEvent.end_minute % 60).padStart(2,'0')}`)" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="Math.floor((editingEvent.end_minute + TIME_OFFSET_MINUTES) / 60) % 24 === (h-1+3)%24 ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ String((h-1+3)%24).padStart(2,'0') }}</button>
                 </div>
                 <div class="flex-1 overflow-y-auto no-scrollbar flex flex-col gap-1 text-center">
                    <button v-for="m in 60" :key="m" @click="editingEvent.end_minute = Math.floor(editingEvent.end_minute / 60) * 60 + (m-1); isEndTimeOpen = false" class="py-2 text-xs rounded-lg transition-colors w-full text-center" :class="editingEvent.end_minute % 60 === (m-1) ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ String(m-1).padStart(2,'0') }}</button>
                 </div>
              </div>
            </div>
          </div>
          <div class="space-y-4">
            <div class="space-y-2"><label class="text-[10px] font-bold text-text-sec">主标签</label>
              <div class="grid grid-cols-4 gap-2"><button v-for="tag in mainTags" :key="tag.id" @click="editingEvent.main_tag_id = tag.id; editingEvent.sub_tag_id = null" class="px-2 py-2 rounded-xl text-[12px] font-bold border-2" :style="{ backgroundColor: editingEvent.main_tag_id === tag.id ? tag.color : 'transparent', borderColor: tag.color, color: editingEvent.main_tag_id === tag.id ? 'white' : tag.color }">{{ tag.name }}</button></div>
            </div>
            <div v-if="editingEvent.main_tag_id && getSubTags(editingEvent.main_tag_id).length" class="space-y-2"><label class="text-[10px] font-bold text-text-sec">副标签</label>
              <div class="flex flex-wrap gap-2"><button v-for="sub in getSubTags(editingEvent.main_tag_id)" :key="sub.id" @click="editingEvent.sub_tag_id = sub.id" class="px-3 py-1.5 rounded-lg text-[12px] font-medium" :class="editingEvent.sub_tag_id === sub.id ? 'bg-[#1D1D1F] text-white' : 'bg-bg-input text-text-sec'">{{ sub.name }}</button></div>
            </div>
            <textarea v-model="editingEvent.content" placeholder="记录具体内容..." @keydown.ctrl.enter="saveEvent" class="w-full bg-bg-input rounded-2xl p-4 text-sm min-h-25 outline-none border border-transparent focus:bg-bg-card focus:border-border-main transition-all"></textarea>
          </div>
          <div class="flex gap-4 pt-4">
             <button v-if="editingEvent.id" @click="deleteEvent(editingEvent.id); isEventModalOpen = false" class="text-[#FF3B30] font-bold px-4 py-2 flex items-center gap-2"><Trash2 :size="18" /> 删除</button>
             <button @click="saveEvent" class="flex-1 bg-[#007AFF] text-white py-4 rounded-2xl font-bold shadow-lg shadow-[#007AFF]/20">保存事件</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Fullscreen & Tooltip & Toast -->
    <div v-if="isFullscreen && previewSrc" class="fixed inset-0 z-200 bg-black/95 flex items-center justify-center p-6 backdrop-blur-xl"><button @click="isFullscreen = false" class="absolute top-10 right-10 w-12 h-12 bg-white/10 rounded-full flex items-center justify-center"><X :size="32" class="text-white" /></button><img :src="previewSrc" class="max-w-full max-h-full object-contain shadow-2xl" /></div>
    <div v-if="hoveredEventDetails && hoveredEventDetails.event" class="fixed z-300 pointer-events-none bg-bg-card/90 backdrop-blur-xl border border-white/20 shadow-2xl rounded-2xl p-4 w-64" :style="{ left: hoveredEventDetails.x + 15 + 'px', top: hoveredEventDetails.y + 15 + 'px' }">
      <div class="flex items-center gap-2 mb-2">
        <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: getTagColor(hoveredEventDetails.event.main_tag_id) }"></div>
        <span class="font-bold text-sm text-text-main">{{ getTagName(hoveredEventDetails.event.main_tag_id) }}</span>
        <span v-if="hoveredEventDetails.event.sub_tag_id" class="text-[10px] font-bold text-text-sec bg-bg-input px-2 py-0.5 rounded-md border border-border-main/50">{{ getTagName(hoveredEventDetails.event.sub_tag_id) }}</span>
      </div>
      <div class="flex items-center justify-between text-[11px] font-bold text-text-sec mb-2.5"><span>{{ logicalMinutesToTime(hoveredEventDetails.event.start_minute) }} - {{ logicalMinutesToTime(hoveredEventDetails.event.end_minute) }}</span><span class="bg-bg-input px-1.5 py-0.5 rounded-lg text-[#007AFF]">{{ formatDuration(hoveredEventDetails.event.start_minute, hoveredEventDetails.event.end_minute) }}</span></div>
      <div v-if="hoveredEventDetails.event.content" class="text-xs text-text-main leading-relaxed wrap-break-words whitespace-pre-wrap">{{ hoveredEventDetails.event.content }}</div>
    </div>
    <div v-if="toast.visible" class="fixed bottom-10 left-1/2 -translate-x-1/2 z-300 animate-in fade-in slide-in-from-bottom-4 duration-300">
      <div class="px-6 py-3 rounded-2xl shadow-2xl backdrop-blur-md flex items-center gap-3 border border-white/20" :class="toast.type === 'error' ? 'bg-[#FF3B30] text-white' : 'bg-bg-card/90 text-text-main'">
        <div v-if="toast.type === 'error'" class="w-5 h-5 rounded-full border-2 border-white flex items-center justify-center text-[12px] font-black">!</div>
        <div v-else class="w-5 h-5 rounded-full bg-[#34C759] flex items-center justify-center text-white text-[10px]">✓</div>
        <span class="text-sm font-bold">{{ toast.message }}</span>
      </div>
    </div>
  </div>
</template>

<style>
.no-scrollbar::-webkit-scrollbar { display: none; }
.no-scrollbar { -ms-overflow-style: none; scrollbar-width: none; }
</style>
