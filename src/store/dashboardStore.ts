import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { DBEvent } from "../types";
import { currentDate, getTagColor, getTagName, formatMinutes, parseISODate, toISODate, viewMode, refreshSignal, dbPath } from "./index";

// Re-export for easier access in Dashboard component
export { getTagColor, getTagName, formatMinutes };

export const dashboardRange = ref<'today' | '7days' | '30days' | 'custom'>('today');
export const dashboardStartDate = ref(currentDate.value);
export const dashboardEndDate = ref(currentDate.value);

// Dedicated memory for custom dates so they don't get overwritten by auto modes
export const customStartDate = ref(currentDate.value);
export const customEndDate = ref(currentDate.value);

export const isStartCalendarOpen = ref(false);
export const isEndCalendarOpen = ref(false);
const rangeEvents = ref<DBEvent[]>([]);
export const excludedDates = ref<string[]>([]);
export const dashboardEvents = computed(() => {
  const excluded = new Set(excludedDates.value);
  return rangeEvents.value.filter(event => !excluded.has(event.date));
});
export const dashboardRangeDays = computed(() => {
  const start = Date.parse(dashboardStartDate.value);
  const end = Date.parse(dashboardEndDate.value);
  return Math.max(0, Math.floor((end - start) / 86400000) + 1);
});
export const activeExcludedDates = computed(() => excludedDates.value.filter(
  date => date >= dashboardStartDate.value && date <= dashboardEndDate.value
));
export const dailyAverageMode = ref<'natural' | 'recorded'>('natural');

let dashboardRequestId = 0;
export const loadDashboardEvents = async () => {
  if (!dbPath.value) return; // Wait until DB path is initialized
  const requestId = ++dashboardRequestId;
  const events = await invoke<DBEvent[]>("get_events_range", { startDate: dashboardStartDate.value, endDate: dashboardEndDate.value });
  if (requestId === dashboardRequestId) rangeEvents.value = events;
};

watch([dashboardRange, dashboardStartDate, dashboardEndDate], () => {
  excludedDates.value = [];
}, { flush: 'sync' });

watch([dashboardRange, currentDate], () => {
  if (dashboardRange.value === 'custom') {
    dashboardStartDate.value = customStartDate.value;
    dashboardEndDate.value = customEndDate.value;
    loadDashboardEvents();
    return;
  }

  const end = parseISODate(currentDate.value);
  let start = parseISODate(currentDate.value);
  
  if (dashboardRange.value === '7days') {
    start.setDate(end.getDate() - 6);
  } else if (dashboardRange.value === '30days') {
    start.setDate(end.getDate() - 29);
  }
  
  dashboardStartDate.value = toISODate(start);
  dashboardEndDate.value = toISODate(end);
  loadDashboardEvents();
}, { immediate: true });

// Specific trigger for custom range changes
watch([customStartDate, customEndDate], () => {
  if (dashboardRange.value === 'custom') {
    dashboardStartDate.value = customStartDate.value;
    dashboardEndDate.value = customEndDate.value;
    loadDashboardEvents();
  }
});

// Auto-refresh dashboard when a refresh signal is received (event added/deleted)
watch(refreshSignal, () => {
  if (viewMode.value === 'dashboard') {
    loadDashboardEvents();
  }
});

// Refresh when switching to dashboard mode to catch updates that happened in preview mode
watch(viewMode, (mode) => {
  if (mode === 'dashboard') {
    loadDashboardEvents();
  }
});

export const dashboardStats = computed(() => {
  let totalMinutes = 0;
  let missingSubTagTotalMinutes = 0;
  const mainTagMap = new Map<number, { total: number, subTags: Map<number, number> }>();
  const missingSubTagMap = new Map<number, { total: number, events: DBEvent[] }>();
  const uniqueDays = new Set<string>();

  dashboardEvents.value.forEach(ev => {
    const diff = Math.max(0, ev.end_minute - ev.start_minute);
    totalMinutes += diff;
    uniqueDays.add(ev.date);

    if (!mainTagMap.has(ev.main_tag_id)) {
      mainTagMap.set(ev.main_tag_id, { total: 0, subTags: new Map() });
    }
    const mainStat = mainTagMap.get(ev.main_tag_id)!;
    mainStat.total += diff;

    if (ev.sub_tag_id) {
      const subTotal = mainStat.subTags.get(ev.sub_tag_id) || 0;
      mainStat.subTags.set(ev.sub_tag_id, subTotal + diff);
    } else {
      missingSubTagTotalMinutes += diff;
      if (!missingSubTagMap.has(ev.main_tag_id)) {
        missingSubTagMap.set(ev.main_tag_id, { total: 0, events: [] });
      }
      const missingStat = missingSubTagMap.get(ev.main_tag_id)!;
      missingStat.total += diff;
      missingStat.events.push(ev);
    }
  });

  const naturalDays = Math.max(0, dashboardRangeDays.value - activeExcludedDates.value.length);
  const recordedDays = uniqueDays.size;
  const daysCount = dailyAverageMode.value === 'natural' ? naturalDays : recordedDays;

  const mainTagsList = Array.from(mainTagMap.entries()).map(([id, stat]) => {
    const subTagsList = Array.from(stat.subTags.entries()).map(([subId, subTotal]) => ({
      id: subId,
      total: subTotal,
      percentage: stat.total > 0 ? (subTotal / stat.total) * 100 : 0
    })).sort((a, b) => b.total - a.total);

    return {
      id,
      total: stat.total,
      dailyAverage: daysCount > 0 ? stat.total / daysCount : 0,
      percentage: totalMinutes > 0 ? (stat.total / totalMinutes) * 100 : 0,
      subTags: subTagsList
    };
  }).sort((a, b) => b.total - a.total);

  const missingSubTags = Array.from(missingSubTagMap.entries()).map(([id, stat]) => ({
    id,
    total: stat.total,
    percentage: totalMinutes > 0 ? (stat.total / totalMinutes) * 100 : 0,
    events: stat.events.sort((a, b) => a.date.localeCompare(b.date) || a.start_minute - b.start_minute)
  })).sort((a, b) => b.total - a.total);

  return {
    totalMinutes,
    dailyAverage: daysCount > 0 ? totalMinutes / daysCount : 0,
    mainTags: mainTagsList,
    naturalDays,
    recordedDays,
    daysCount,
    missingSubTagTotalMinutes,
    missingSubTagCount: missingSubTags.reduce((sum, tag) => sum + tag.events.length, 0),
    missingSubTags
  };
});
