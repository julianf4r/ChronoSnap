import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Image } from "@tauri-apps/api/image";
import { Tag, DBEvent, TimelineItem, Toast, Reminder, PlanTask } from "../types";

export const TIME_OFFSET_MINUTES = 180;
export const TOTAL_MINUTES = 1440;

// --- Date Utils (Logical Day Safe) ---
export const getLogicDateStr = (date: Date = new Date()) => {
  // Subtract offset to get logic day
  const d = new Date(date.getTime() - TIME_OFFSET_MINUTES * 60000);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
};

export const parseISODate = (dateStr: string) => {
  const [y, m, d] = dateStr.split('-').map(Number);
  return new Date(y, m - 1, d, 12, 0, 0); 
};

export const toISODate = (date: Date) => {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
};

// --- Config State ---
export const isSetupComplete = ref(false);
export const savePath = ref("");
export const dbPath = ref("");
export const retainDays = ref(30);
export const captureInterval = ref(60);
export const timelineZoom = ref(1.5);
export const theme = ref("system");

// --- Global UI State ---
export const isPaused = ref(false);
export const currentDate = ref(getLogicDateStr()); // Initialize with logic date
export const todayLogicalDate = ref(getLogicDateStr());
export const currentLogicalMinute = ref(-1); // Exported to keep track of current minute globally
export const viewMode = ref<'preview' | 'plan' | 'dashboard'>('plan');
export const isFullscreen = ref(false);
export const previewSrc = ref("");
export const selectedImage = ref<TimelineItem | null>(null);
export const lockedImage = ref<TimelineItem | null>(null);

// --- Data State ---
export const tags = ref<Tag[]>([]);
export const dayEvents = ref<DBEvent[]>([]);
export const reminders = ref<Reminder[]>([]);
export const planTasks = ref<PlanTask[]>([]);
export const calendarStatus = ref<Record<string, { has_overdue: boolean, has_upcoming: boolean }>>({});
export const timelineImages = ref<TimelineItem[]>([]);
export const refreshSignal = ref(0); // Counter to trigger dashboard refreshes

// --- Global Toast ---
export const toast = ref<Toast>({ message: "", type: "success", visible: false });
export const showToast = (message: string, type: "success" | "error" = "success") => {
  toast.value = { message, type, visible: true };
  setTimeout(() => { toast.value.visible = false; }, 3000);
};

// --- Computed Helpers ---
export const mainTags = computed(() => tags.value.filter(t => t.parent_id === null));
export const getSubTags = (parentId: number) => tags.value.filter(t => t.parent_id === parentId);
export const getTagColor = (tagId: number | null | undefined) => {
  if (tagId == null) return "#007AFF";
  const tag = tags.value.find(t => t.id === tagId);
  return tag?.color || "#007AFF";
};
export const getTagName = (tagId: number | null | undefined) => {
  if (tagId == null) return "";
  const tag = tags.value.find(t => t.id === tagId);
  return tag?.name || "";
};

// --- Time Helper Functions ---
export const logicalMinutesToTime = (min: number) => {
  let t = (min + TIME_OFFSET_MINUTES) % 1440;
  const h = Math.floor(t / 60); const m = Math.floor(t % 60);
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
};

export const logicalMinutesFromTime = (timeStr: string) => {
  const [h, m] = timeStr.split(":").map(Number);
  let total = h * 60 + m;
  if (h < 3) total += 1440;
  return total - TIME_OFFSET_MINUTES;
};

export const formatDuration = (start: number, end: number) => {
  let diff = end - start;
  if (diff < 0) diff += 1440;
  const h = Math.floor(diff / 60);
  const m = diff % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
};

export const formatMinutes = (mins: number) => {
  const h = Math.floor(mins / 60);
  const m = Math.floor(mins % 60);
  if (h === 0) return `${m}m`;
  return `${h}h ${m}m`;
};

// --- Shared Actions ---
export const loadTags = async () => { tags.value = await invoke("get_tags"); };
export const loadEvents = async () => { 
  dayEvents.value = await invoke("get_events", { date: currentDate.value }); 
  refreshSignal.value++; // Increment to signal other stores to refresh
};
export const loadReminders = async () => {
  reminders.value = await invoke("get_reminders", { date: currentDate.value });
  refreshBadgeCount();
  refreshCalendarStatus(); // 提醒更新时也刷新日历状态
};
export const loadPlanTasks = async () => {
  if (!dbPath.value) return;
  planTasks.value = await invoke("get_plan_tasks");
};

export const loadCalendarStatus = async (yearMonth: string) => {
  if (!dbPath.value) return;
  try {
    const { date, minute } = getRealNowLogicalTime();
    const statuses: any[] = await invoke("get_reminders_by_month", { 
      yearMonth, 
      today: date, 
      nowMinute: minute 
    });
    const map: Record<string, any> = {};
    statuses.forEach(s => { map[s.date] = s; });
    calendarStatus.value = map;
  } catch (e) {
    console.error("Failed to load calendar status:", e);
  }
};

// 当前显示的日历月份，用于自动刷新状态
export const currentCalendarMonthStr = ref("");

export const refreshCalendarStatus = () => {
  if (currentCalendarMonthStr.value) {
    loadCalendarStatus(currentCalendarMonthStr.value);
  }
};

export const overdueCount = ref(0);

// 获取当前真实的逻辑时间和日期（不依赖 UI 状态）
export const getRealNowLogicalTime = () => {
  const now = new Date();
  const d = new Date(now.getTime() - TIME_OFFSET_MINUTES * 60000);
  const date = toISODate(d);
  const m = now.getHours() * 60 + now.getMinutes();
  const minute = (m < TIME_OFFSET_MINUTES ? m + 1440 : m) - TIME_OFFSET_MINUTES;
  return { date, minute };
};

export const refreshBadgeCount = async () => {
  if (!dbPath.value) return; 
  try {
    const { date, minute } = getRealNowLogicalTime();
    const count: number = await invoke("get_overdue_reminders_count", { 
      date: date, 
      minute: minute 
    });
    overdueCount.value = count;
    await updateTaskbarBadge(count);
  } catch (e) {
    console.error("Failed to refresh badge count:", e);
  }
};


const updateTaskbarBadge = async (count: number) => {
  try {
    const win = getCurrentWindow();
    if (count <= 0) {
      await win.setOverlayIcon(undefined);
      return;
    }

    const canvas = document.createElement('canvas');
    const size = 64; 
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;

    // Draw red circle
    ctx.fillStyle = '#FF3B30';
    ctx.beginPath();
    ctx.arc(size/2, size/2, size/2 - 2, 0, Math.PI * 2);
    ctx.fill();

    // Draw white text
    ctx.fillStyle = '#FFFFFF';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = 'bold 38px Arial';
    const displayCount = count > 99 ? '99+' : count.toString();
    if (displayCount.length > 2) ctx.font = 'bold 28px Arial';
    ctx.fillText(displayCount, size/2, size/2 + 2);

    // Get raw RGBA pixels and wrap in Tauri Image object
    const imageData = ctx.getImageData(0, 0, size, size);
    const img = await Image.new(new Uint8Array(imageData.data), size, size);

    await win.setOverlayIcon(img);
  } catch (e) {
    console.error("Failed to set overlay icon:", e);
  }
};
