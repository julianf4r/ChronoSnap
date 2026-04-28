<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { X, Calendar, ChevronLeft, ChevronRight, Copy, ArrowDown, ArrowUp, Download } from "lucide-vue-next";
import { currentDate, showToast, getTagName, logicalMinutesToTime, toISODate, mainTags } from "../../store";
import { DBEvent } from "../../types";

defineEmits(['close']);

const startCalendarRef = ref<HTMLElement | null>(null);
const endCalendarRef = ref<HTMLElement | null>(null);

const exportStartDate = ref(new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toLocaleDateString('sv'));
const exportEndDate = ref(currentDate.value);
const exportStartMonth = ref(new Date(exportStartDate.value));
const exportEndMonth = ref(new Date(exportEndDate.value));
const isExportStartCalendarOpen = ref(false);
const isExportEndCalendarOpen = ref(false);

// 选择的主标签，默认全选
const selectedTags = ref<number[]>(mainTags.value.map(t => t.id));

// 表格数据
const dateList = ref<string[]>([]);
const previewData = ref<Record<string, Record<number, string>>>({});
const isDesc = ref(true);
const sortedDateList = computed(() => isDesc.value ? [...dateList.value].reverse() : dateList.value);

// 拖拽调整宽高状态
const colWidths = ref<Record<string | number, number>>({});
const rowHeights = ref<Record<string, number>>({});
const isDragging = ref(false);

let dragType: 'col' | 'row' | null = null;
let dragKey: string | number | null = null;
let startPos = 0;
let startSize = 0;

const MIN_COL_WIDTH = 120;
const MIN_ROW_HEIGHT = 48;

const startResizeCol = (e: MouseEvent, key: string | number) => {
  isDragging.value = true;
  dragType = 'col';
  dragKey = key;
  startPos = e.clientX;
  startSize = colWidths.value[key] || MIN_COL_WIDTH;
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
};

const startResizeRow = (e: MouseEvent, key: string) => {
  isDragging.value = true;
  dragType = 'row';
  dragKey = key;
  startPos = e.clientY;
  startSize = rowHeights.value[key] || (e.target as HTMLElement).closest('td')?.offsetHeight || MIN_ROW_HEIGHT;
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
};

const onMouseMove = (e: MouseEvent) => {
  if (!isDragging.value || dragKey === null) return;
  if (dragType === 'col') {
    const delta = e.clientX - startPos;
    colWidths.value[dragKey] = Math.max(MIN_COL_WIDTH, startSize + delta);
  } else if (dragType === 'row') {
    const delta = e.clientY - startPos;
    rowHeights.value[dragKey as string] = Math.max(MIN_ROW_HEIGHT, startSize + delta);
  }
};

const onMouseUp = () => {
  isDragging.value = false;
  dragType = null;
  dragKey = null;
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('mouseup', onMouseUp);
};

const toggleTag = (id: number) => {
  if (selectedTags.value.includes(id)) {
    selectedTags.value = selectedTags.value.filter(tId => tId !== id);
  } else {
    selectedTags.value.push(id);
  }
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  if (isExportStartCalendarOpen.value && startCalendarRef.value && !startCalendarRef.value.contains(target)) {
    isExportStartCalendarOpen.value = false;
  }
  if (isExportEndCalendarOpen.value && endCalendarRef.value && !endCalendarRef.value.contains(target)) {
    isExportEndCalendarOpen.value = false;
  }
};

onMounted(() => {
  window.addEventListener('mousedown', handleClickOutside);
});

onUnmounted(() => {
  window.removeEventListener('mousedown', handleClickOutside);
  document.removeEventListener('mousemove', onMouseMove);
  document.removeEventListener('mouseup', onMouseUp);
});

const exportStartCalendarDays = computed(() => {
  const y = exportStartMonth.value.getFullYear();
  const m = exportStartMonth.value.getMonth();
  const firstDay = new Date(y, m, 1).getDay();
  const daysInMonth = new Date(y, m + 1, 0).getDate();
  const days = [];
  const padding = (firstDay + 6) % 7;
  for (let i = 0; i < padding; i++) days.push(null);
  for (let i = 1; i <= daysInMonth; i++) days.push(new Date(y, m, i));
  return days;
});

const exportEndCalendarDays = computed(() => {
  const y = exportEndMonth.value.getFullYear();
  const m = exportEndMonth.value.getMonth();
  const firstDay = new Date(y, m, 1).getDay();
  const daysInMonth = new Date(y, m + 1, 0).getDate();
  const days = [];
  const padding = (firstDay + 6) % 7;
  for (let i = 0; i < padding; i++) days.push(null);
  for (let i = 1; i <= daysInMonth; i++) days.push(new Date(y, m, i));
  return days;
});

const parseDateStr = (str: string) => {
  const [y, m, d] = str.split('-').map(Number);
  return new Date(y, m - 1, d);
};

const handlePreview = async () => {
  if (selectedTags.value.length === 0) {
    showToast("请至少选择一个主标签", "error");
    return;
  }

  const curDate = parseDateStr(exportStartDate.value);
  const endDate = parseDateStr(exportEndDate.value);
  
  if (curDate > endDate) {
    showToast("开始日期不能晚于结束日期", "error");
    return;
  }

  try {
    const events: DBEvent[] = await invoke("get_events_range", { startDate: exportStartDate.value, endDate: exportEndDate.value });
    
    // 生成日期范围列表
    const dates: string[] = [];
    let tempDate = new Date(curDate);
    while (tempDate <= endDate) {
      dates.push(toISODate(tempDate));
      tempDate.setDate(tempDate.getDate() + 1);
    }
    dateList.value = dates;

    // 初始化列宽
    colWidths.value = { date: 150 };
    selectedTags.value.forEach(tagId => {
      colWidths.value[tagId] = 200;
    });
    // 初始化统一行高
    rowHeights.value = {};
    dates.forEach(d => {
      rowHeights.value[d] = 100;
    });

    // 过滤选中的主标签并构建矩阵
    const matrix: Record<string, Record<number, string>> = {};
    dates.forEach(d => matrix[d] = {});

    const filteredEvents = events.filter(e => selectedTags.value.includes(e.main_tag_id));

    filteredEvents.forEach(e => {
       if (!matrix[e.date]) matrix[e.date] = {}; // 容错
       
       const cellArr = matrix[e.date][e.main_tag_id] ? matrix[e.date][e.main_tag_id].split('\n') : [];
       const startTime = logicalMinutesToTime(e.start_minute);
       const endTime = logicalMinutesToTime(e.end_minute);
       const subTag = getTagName(e.sub_tag_id);
       // 替换换行符为空格
       const content = e.content ? e.content.replace(/\n/g, ' ') : '';

       let line = `${startTime}-${endTime}`;
       if (subTag) line += ` ${subTag}`;
       if (content) line += ` ${content}`;

       cellArr.push(line);
       matrix[e.date][e.main_tag_id] = cellArr.join('\n');
    });

    previewData.value = matrix;

  } catch (e) {
    showToast("获取数据失败: " + e, "error");
  }
};

const exportToJson = async () => {
  if (selectedTags.value.length === 0) {
    showToast("请至少选择一个主标签", "error");
    return;
  }

  const curDate = parseDateStr(exportStartDate.value);
  const endDate = parseDateStr(exportEndDate.value);
  if (curDate > endDate) {
    showToast("开始日期不能晚于结束日期", "error");
    return;
  }

  try {
    const events: DBEvent[] = await invoke("get_events_range", { startDate: exportStartDate.value, endDate: exportEndDate.value });
    const filteredEvents = events.filter(e => selectedTags.value.includes(e.main_tag_id));
    
    // Add tag names to the exported data for better readability
    const exportData = filteredEvents.map(e => ({
      ...e,
      main_tag_name: getTagName(e.main_tag_id),
      sub_tag_name: getTagName(e.sub_tag_id),
      start_time_str: logicalMinutesToTime(e.start_minute),
      end_time_str: logicalMinutesToTime(e.end_minute),
    }));

    const jsonStr = JSON.stringify(exportData, null, 2);
    const filePath = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: `chrono-snap-export-${exportStartDate.value}-to-${exportEndDate.value}.json`
    });

    if (filePath) {
      await invoke("write_file", { path: filePath, content: jsonStr });
      showToast("导出成功");
    }
  } catch (e) {
    showToast("导出失败: " + e, "error");
  }
};

const copyColumn = async (tagId: number | 'date') => {
  let lines: string[] = [];
  
  for (const date of sortedDateList.value) {
    if (tagId === 'date') {
      lines.push(date);
    } else {
      let cellStr = previewData.value[date][tagId] || "";
      if (cellStr.includes('\n') || cellStr.includes('\t') || cellStr.includes('"')) {
         cellStr = `"${cellStr.replace(/"/g, '""')}"`;
      }
      lines.push(cellStr);
    }
  }

  try {
    await navigator.clipboard.writeText(lines.join("\n"));
    showToast(`列数据已复制`);
  } catch(err) {
    showToast("复制失败", "error");
  }
};

const copyToClipboard = async () => {
  const header = ["日期", ...selectedTags.value.map(id => getTagName(id))];
  let tsv = header.join("\t") + "\n";
  
  for (const date of sortedDateList.value) {
    let row = [date];
    for (const tagId of selectedTags.value) {
      let cellStr = previewData.value[date][tagId] || "";
      if (cellStr.includes('\n') || cellStr.includes('\t') || cellStr.includes('"')) {
         cellStr = `"${cellStr.replace(/"/g, '""')}"`;
      }
      row.push(cellStr);
    }
    tsv += row.join("\t") + "\n";
  }

  try {
    await navigator.clipboard.writeText(tsv);
    showToast("表格已复制，可直接粘贴到 Excel");
  } catch(err) {
    showToast("复制失败", "error");
  }
};
</script>

<template>
  <div class="fixed inset-0 z-100 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="$emit('close')">
    <div class="bg-bg-card rounded-[40px] shadow-2xl w-full flex flex-col animate-in fade-in zoom-in duration-300 transition-all" 
         :class="dateList.length > 0 ? 'max-w-[90vw] md:max-w-6xl h-[90vh] overflow-hidden' : 'max-w-4xl min-h-125'">
      <div class="p-8 border-b flex justify-between items-center shrink-0">
        <h2 class="text-2xl font-bold">导出记录</h2>
        <button @click="$emit('close')" class="hover:bg-bg-input p-2 rounded-full transition-colors"><X :size="24" /></button>
      </div>
      
      <div class="p-8 flex-1 flex flex-col gap-8 no-scrollbar" :class="dateList.length > 0 ? 'overflow-y-auto' : 'overflow-visible'">
        <!-- 配置区域 -->
        <div class="space-y-6 shrink-0">
          <!-- 日期选择 -->
          <div class="space-y-2">
            <label class="text-[11px] font-bold text-text-sec">日期范围</label>
            <div class="flex gap-4 items-center">
              <div class="relative flex-1" ref="startCalendarRef">
                <button @click="isExportStartCalendarOpen = !isExportStartCalendarOpen; isExportEndCalendarOpen = false" class="w-full bg-bg-input rounded-xl pl-10 pr-4 py-3 text-sm outline-none border border-transparent focus:border-[#007AFF] transition-all text-left flex items-center">
                  <Calendar :size="16" class="absolute left-4 text-text-sec" />
                  {{ exportStartDate }}
                </button>
                <div v-if="isExportStartCalendarOpen" class="absolute top-full left-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-5 animate-in fade-in zoom-in-95 duration-200 w-72">
                  <div class="flex items-center justify-between mb-4">
                    <button @click="exportStartMonth = new Date(exportStartMonth.getFullYear(), exportStartMonth.getMonth()-1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronLeft :size="18"/></button>
                    <span class="text-sm font-black">{{ exportStartMonth.getFullYear() }}年 {{ exportStartMonth.getMonth()+1 }}月</span>
                    <button @click="exportStartMonth = new Date(exportStartMonth.getFullYear(), exportStartMonth.getMonth()+1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronRight :size="18"/></button>
                  </div>
                  <div class="grid grid-cols-7 gap-1 text-center mb-2"><div v-for="d in ['一','二','三','四','五','六','日']" :key="d" class="text-[10px] font-bold text-text-sec">{{d}}</div></div>
                  <div class="grid grid-cols-7 gap-1">
                    <div v-for="(date, i) in exportStartCalendarDays" :key="i" class="aspect-square flex items-center justify-center">
                      <button v-if="date" @click="exportStartDate = toISODate(date); isExportStartCalendarOpen = false" class="w-8 h-8 rounded-full text-xs font-medium transition-all" :class="toISODate(date) === exportStartDate ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ date.getDate() }}</button>
                    </div>
                  </div>
                </div>
              </div>
              <span class="text-text-sec font-black text-xs">至</span>
              <div class="relative flex-1" ref="endCalendarRef">
                <button @click="isExportEndCalendarOpen = !isExportEndCalendarOpen; isExportStartCalendarOpen = false" class="w-full bg-bg-input rounded-xl pl-10 pr-4 py-3 text-sm outline-none border border-transparent focus:border-[#007AFF] transition-all text-left flex items-center">
                  <Calendar :size="16" class="absolute left-4 text-text-sec" />
                  {{ exportEndDate }}
                </button>
                <div v-if="isExportEndCalendarOpen" class="absolute top-full right-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-5 animate-in fade-in zoom-in-95 duration-200 w-72">
                  <div class="flex items-center justify-between mb-4">
                    <button @click="exportEndMonth = new Date(exportEndMonth.getFullYear(), exportEndMonth.getMonth()-1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronLeft :size="18"/></button>
                    <span class="text-sm font-black">{{ exportEndMonth.getFullYear() }}年 {{ exportEndMonth.getMonth()+1 }}月</span>
                    <button @click="exportEndMonth = new Date(exportEndMonth.getFullYear(), exportEndMonth.getMonth()+1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronRight :size="18"/></button>
                  </div>
                  <div class="grid grid-cols-7 gap-1 text-center mb-2"><div v-for="d in ['一','二','三','四','五','六','日']" :key="d" class="text-[10px] font-bold text-text-sec">{{d}}</div></div>
                  <div class="grid grid-cols-7 gap-1">
                    <div v-for="(date, i) in exportEndCalendarDays" :key="i" class="aspect-square flex items-center justify-center">
                      <button v-if="date" @click="exportEndDate = toISODate(date); isExportEndCalendarOpen = false" class="w-8 h-8 rounded-full text-xs font-medium transition-all" :class="toISODate(date) === exportEndDate ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ date.getDate() }}</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 主标签选择 -->
          <div class="space-y-2">
            <label class="text-[11px] font-bold text-text-sec">选择要导出的主标签</label>
            <div class="flex flex-wrap gap-2">
              <button v-for="tag in mainTags" :key="tag.id"
                @click="toggleTag(tag.id)"
                class="px-3 py-1.5 rounded-xl text-[12px] font-bold border-2 transition-all active:scale-95"
                :style="selectedTags.includes(tag.id) ? { backgroundColor: tag.color, borderColor: tag.color, color: 'white' } : { borderColor: tag.color, color: tag.color }"
              >
                {{ tag.name }}
              </button>
            </div>
          </div>

          <div class="flex gap-4">
            <button @click="exportToJson" :disabled="!exportStartDate || !exportEndDate" class="flex-1 bg-bg-input border border-border-main text-text-sec hover:bg-border-main hover:text-text-main py-3.5 rounded-2xl font-bold shadow-sm transition-all disabled:opacity-50 flex items-center justify-center gap-2">
              <Download :size="16" /> 导出为 JSON
            </button>
            <button @click="handlePreview" :disabled="!exportStartDate || !exportEndDate" class="flex-1 bg-[#007AFF] text-white py-3.5 rounded-2xl font-bold shadow-lg shadow-[#007AFF]/20 active:scale-95 transition-all disabled:opacity-50 disabled:active:scale-100">
              生成预览表格
            </button>
          </div>
        </div>

        <!-- 表格预览区域 -->
        <div v-if="dateList.length > 0" class="flex-1 flex flex-col border-t border-border-main pt-6">
          <div class="flex justify-between items-center mb-4 shrink-0">
            <div class="flex items-center gap-4">
              <h3 class="text-sm font-bold text-text-sec flex items-center gap-2">数据预览</h3>
              <button @click="isDesc = !isDesc" class="flex items-center gap-1.5 px-3 py-1.5 bg-bg-input hover:bg-border-main text-text-sec hover:text-text-main rounded-lg text-xs font-bold transition-all active:scale-95">
                <ArrowDown v-if="!isDesc" :size="14" />
                <ArrowUp v-else :size="14" />
                {{ isDesc ? '倒序 (由近及远)' : '正序 (由远及近)' }}
              </button>
            </div>
            <button @click="copyToClipboard" class="flex items-center gap-1.5 px-4 py-2 bg-[#007AFF] hover:bg-[#007AFF]/90 text-white rounded-xl text-xs font-bold transition-all shadow-lg shadow-[#007AFF]/20 active:scale-95">
              <Copy :size="14" /> 一键复制完整表格
            </button>
          </div>
          
          <div class="flex-1 overflow-auto border border-border-main rounded-2xl bg-bg-input/30 relative" :class="isDragging ? 'select-none' : 'select-text'">
            <table class="w-max min-w-full text-left border-collapse text-xs table-fixed">
              <thead class="bg-bg-card sticky top-0 z-30 shadow-sm">
                <tr>
                  <th class="relative p-4 border-b border-border-main font-bold whitespace-nowrap text-text-sec bg-bg-card group" :style="{ width: colWidths['date'] + 'px' }">
                    <div class="flex items-center gap-2 overflow-hidden w-full">
                      日期
                      <button @click="copyColumn('date')" title="复制此列" class="opacity-0 group-hover:opacity-100 p-1 hover:bg-bg-input rounded transition-all text-text-sec shrink-0"><Copy :size="12"/></button>
                    </div>
                    <div class="absolute right-0 top-0 bottom-0 w-1.5 cursor-col-resize hover:bg-[#007AFF] z-40 group-hover:bg-border-main" @mousedown.prevent="startResizeCol($event, 'date')"></div>
                  </th>
                  <th v-for="tagId in selectedTags" :key="tagId" class="relative p-4 border-b border-l border-border-main/50 font-bold whitespace-nowrap bg-bg-card group" :style="{ color: mainTags.find(t => t.id === tagId)?.color || 'inherit', width: colWidths[tagId] + 'px' }">
                    <div class="flex items-center gap-2 overflow-hidden w-full">
                      {{ getTagName(tagId) }}
                      <button @click="copyColumn(tagId)" title="复制此列" class="opacity-0 group-hover:opacity-100 p-1 hover:bg-bg-input rounded transition-all text-text-sec shrink-0"><Copy :size="12"/></button>
                    </div>
                    <div class="absolute right-0 top-0 bottom-0 w-1.5 cursor-col-resize hover:bg-[#007AFF] z-40 group-hover:bg-border-main" @mousedown.prevent="startResizeCol($event, tagId)"></div>
                  </th>
                </tr>
              </thead>
              <tbody class="bg-bg-card">
                <tr v-for="date in sortedDateList" :key="date" class="border-b border-border-main/30 hover:bg-bg-input/50 transition-colors">
                  <td class="relative p-4 font-bold whitespace-nowrap text-text-main align-top group">
                    <div class="flex items-start overflow-hidden w-full" :style="{ height: rowHeights[date] ? Math.max(0, rowHeights[date] - 32) + 'px' : 'auto', minHeight: '16px' }">
                      {{ date }}
                    </div>
                    <div class="absolute bottom-0 left-0 right-0 h-1.5 cursor-row-resize hover:bg-[#007AFF] z-20 group-hover:bg-border-main" @mousedown.prevent="startResizeRow($event, date)"></div>
                  </td>
                  <td v-for="tagId in selectedTags" :key="tagId" class="relative p-4 border-l border-border-main/30 align-top whitespace-pre-wrap leading-relaxed text-text-sec group">
                    <div class="w-full overflow-y-auto no-scrollbar wrap-break-words" :style="{ height: rowHeights[date] ? Math.max(0, rowHeights[date] - 32) + 'px' : 'auto', minHeight: '16px' }">
                      {{ previewData[date]?.[tagId] || '' }}
                    </div>
                    <div class="absolute bottom-0 left-0 right-0 h-1.5 cursor-row-resize hover:bg-[#007AFF] z-20 group-hover:bg-border-main" @mousedown.prevent="startResizeRow($event, date)"></div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>
