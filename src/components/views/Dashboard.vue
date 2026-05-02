<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { BarChart2, Calendar, ChevronLeft, ChevronRight } from "lucide-vue-next";
import { 
  dashboardStats, dashboardRange, dailyAverageMode,
  customStartDate, customEndDate,
  isStartCalendarOpen, isEndCalendarOpen,
  formatMinutes, getTagColor, getTagName 
} from "../../store/dashboardStore";
import { toISODate } from "../../store";
import { logicalMinutesToTime } from "../../store";

const startCalendarRef = ref<HTMLElement | null>(null);
const endCalendarRef = ref<HTMLElement | null>(null);

const startCalendarMonth = ref(new Date(customStartDate.value));
const endCalendarMonth = ref(new Date(customEndDate.value));

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  if (isStartCalendarOpen.value && startCalendarRef.value && !startCalendarRef.value.contains(target)) {
    isStartCalendarOpen.value = false;
  }
  if (isEndCalendarOpen.value && endCalendarRef.value && !endCalendarRef.value.contains(target)) {
    isEndCalendarOpen.value = false;
  }
};

onMounted(() => {
  window.addEventListener('mousedown', handleClickOutside);
});

onUnmounted(() => {
  window.removeEventListener('mousedown', handleClickOutside);
});

const getCalendarDays = (month: Date) => {
  const y = month.getFullYear();
  const m = month.getMonth();
  const firstDay = new Date(y, m, 1).getDay();
  const daysInMonth = new Date(y, m + 1, 0).getDate();
  const days = [];
  const padding = (firstDay + 6) % 7;
  for (let i = 0; i < padding; i++) days.push(null);
  for (let i = 1; i <= daysInMonth; i++) days.push(new Date(y, m, i));
  return days;
};

const startCalendarDays = computed(() => getCalendarDays(startCalendarMonth.value));
const endCalendarDays = computed(() => getCalendarDays(endCalendarMonth.value));
</script>

<template>
  <div class="absolute inset-0 overflow-y-auto no-scrollbar p-10 bg-bg-main animate-in fade-in zoom-in-95 duration-200">
    <div class="max-w-3xl mx-auto">
      <!-- Top Filters -->
      <div class="flex flex-col gap-6 mb-10">
        <div class="flex justify-between items-center">
           <div class="flex bg-bg-input rounded-xl p-1.5 gap-1 border border-border-main/50 shadow-inner">
              <button @click="dashboardRange = 'today'" class="px-5 py-2 text-xs font-bold rounded-lg transition-all" :class="dashboardRange === 'today' ? 'bg-[#007AFF] shadow-lg text-white' : 'text-text-sec hover:text-text-main hover:bg-bg-card'">今日</button>
              <button @click="dashboardRange = '7days'" class="px-5 py-2 text-xs font-bold rounded-lg transition-all" :class="dashboardRange === '7days' ? 'bg-[#007AFF] shadow-lg text-white' : 'text-text-sec hover:text-text-main hover:bg-bg-card'">近 7 天</button>
              <button @click="dashboardRange = '30days'" class="px-5 py-2 text-xs font-bold rounded-lg transition-all" :class="dashboardRange === '30days' ? 'bg-[#007AFF] shadow-lg text-white' : 'text-text-sec hover:text-text-main hover:bg-bg-card'">近 30 天</button>
              <button @click="dashboardRange = 'custom'" class="px-5 py-2 text-xs font-bold rounded-lg transition-all" :class="dashboardRange === 'custom' ? 'bg-[#007AFF] shadow-lg text-white' : 'text-text-sec hover:text-text-main hover:bg-bg-card'">自定义</button>
           </div>
           <div v-if="dashboardRange !== 'today'" class="flex bg-bg-input rounded-xl p-1 gap-1 border border-border-main/50 shadow-inner">
              <button @click="dailyAverageMode = 'natural'" class="px-3 py-1.5 text-[10px] font-bold rounded-lg transition-all" :class="dailyAverageMode === 'natural' ? 'bg-bg-card shadow-sm text-text-main' : 'text-text-sec hover:text-text-main'">自然天数 ({{dashboardStats.naturalDays}}天)</button>
              <button @click="dailyAverageMode = 'recorded'" class="px-3 py-1.5 text-[10px] font-bold rounded-lg transition-all" :class="dailyAverageMode === 'recorded' ? 'bg-bg-card shadow-sm text-text-main' : 'text-text-sec hover:text-text-main'">记录天数 ({{dashboardStats.recordedDays}}天)</button>
           </div>
        </div>

        <!-- Custom Date Pickers -->
        <div v-if="dashboardRange === 'custom'" class="flex items-center gap-4 animate-in slide-in-from-top-2 duration-300">
           <div class="relative flex-1" ref="startCalendarRef">
             <button @click="isStartCalendarOpen = !isStartCalendarOpen; isEndCalendarOpen = false" class="w-full bg-bg-card border border-border-main rounded-xl pl-10 pr-4 py-2.5 text-xs font-bold text-left flex items-center hover:bg-bg-input transition-all">
                <Calendar :size="16" class="absolute left-3.5 text-[#007AFF]" />
                <span class="text-text-sec mr-2">从</span> {{ customStartDate }}
             </button>
             <div v-if="isStartCalendarOpen" class="absolute top-full left-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-5 w-72 animate-in fade-in zoom-in-95">
                <div class="flex items-center justify-between mb-4">
                  <button @click="startCalendarMonth = new Date(startCalendarMonth.getFullYear(), startCalendarMonth.getMonth()-1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronLeft :size="16"/></button>
                  <span class="text-sm font-black">{{ startCalendarMonth.getFullYear() }}年 {{ startCalendarMonth.getMonth()+1 }}月</span>
                  <button @click="startCalendarMonth = new Date(startCalendarMonth.getFullYear(), startCalendarMonth.getMonth()+1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronRight :size="16"/></button>
                </div>
                <div class="grid grid-cols-7 gap-1 text-center mb-2"><div v-for="d in ['一','二','三','四','五','六','日']" :key="d" class="text-[10px] font-bold text-text-sec">{{d}}</div></div>
                <div class="grid grid-cols-7 gap-1">
                  <div v-for="(date, i) in startCalendarDays" :key="i" class="aspect-square flex items-center justify-center">
                    <button v-if="date" @click="customStartDate = toISODate(date); isStartCalendarOpen = false" class="w-8 h-8 rounded-full text-xs font-medium transition-all" :class="toISODate(date) === customStartDate ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ date.getDate() }}</button>
                  </div>
                </div>
             </div>
           </div>
           <div class="text-text-sec font-black text-xs">至</div>
           <div class="relative flex-1" ref="endCalendarRef">
             <button @click="isEndCalendarOpen = !isEndCalendarOpen; isStartCalendarOpen = false" class="w-full bg-bg-card border border-border-main rounded-xl pl-10 pr-4 py-2.5 text-xs font-bold text-left flex items-center hover:bg-bg-input transition-all">
                <Calendar :size="16" class="absolute left-3.5 text-[#007AFF]" />
                <span class="text-text-sec mr-2">至</span> {{ customEndDate }}
             </button>
             <div v-if="isEndCalendarOpen" class="absolute top-full right-0 mt-2 bg-bg-card rounded-3xl shadow-2xl border border-border-main z-120 p-5 w-72 animate-in fade-in zoom-in-95">
                <div class="flex items-center justify-between mb-4">
                  <button @click="endCalendarMonth = new Date(endCalendarMonth.getFullYear(), endCalendarMonth.getMonth()-1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronLeft :size="16"/></button>
                  <span class="text-sm font-black">{{ endCalendarMonth.getFullYear() }}年 {{ endCalendarMonth.getMonth()+1 }}月</span>
                  <button @click="endCalendarMonth = new Date(endCalendarMonth.getFullYear(), endCalendarMonth.getMonth()+1, 1)" class="p-2 hover:bg-bg-input rounded-xl"><ChevronRight :size="16"/></button>
                </div>
                <div class="grid grid-cols-7 gap-1 text-center mb-2"><div v-for="d in ['一','二','三','四','五','六','日']" :key="d" class="text-[10px] font-bold text-text-sec">{{d}}</div></div>
                <div class="grid grid-cols-7 gap-1">
                  <div v-for="(date, i) in endCalendarDays" :key="i" class="aspect-square flex items-center justify-center">
                    <button v-if="date" @click="customEndDate = toISODate(date); isEndCalendarOpen = false" class="w-8 h-8 rounded-full text-xs font-medium transition-all" :class="toISODate(date) === customEndDate ? 'bg-[#007AFF] text-white font-bold' : 'hover:bg-bg-input text-main'">{{ date.getDate() }}</button>
                  </div>
                </div>
             </div>
           </div>
        </div>
      </div>

      <!-- Overview Cards -->
      <div class="grid grid-cols-2 gap-6 mb-10">
         <div class="bg-bg-card border border-border-main rounded-3xl p-6 shadow-sm flex flex-col justify-center">
            <div class="text-xs font-bold text-text-sec mb-2">总记录时长</div>
            <div class="text-4xl font-black text-text-main">{{ formatMinutes(dashboardStats.totalMinutes) }}</div>
         </div>
         <div class="bg-bg-card border border-border-main rounded-3xl p-6 shadow-sm flex flex-col justify-center">
            <div class="text-xs font-bold text-text-sec mb-2">日均时长</div>
            <div class="text-4xl font-black text-[#007AFF]">{{ formatMinutes(dashboardStats.dailyAverage) }}</div>
         </div>
      </div>

      <!-- Visual Bar -->
      <div class="mb-12 bg-bg-card border border-border-main p-6 rounded-3xl shadow-sm">
         <h3 class="text-sm font-bold text-text-sec mb-4 uppercase tracking-wider">占比总览</h3>
         <div class="h-6 w-full bg-bg-input rounded-full overflow-hidden flex shadow-inner mb-4 border border-border-main/30">
            <div v-for="tag in dashboardStats.mainTags" :key="tag.id" :style="{ width: tag.percentage + '%', backgroundColor: getTagColor(tag.id) }" class="h-full first:rounded-l-full last:rounded-r-full border-r border-bg-main/20 last:border-0 transition-all duration-500 hover:brightness-110 cursor-pointer" :title="getTagName(tag.id) + ' ' + tag.percentage.toFixed(1) + '%'"></div>
         </div>
         <div class="flex flex-wrap gap-4 px-2">
            <div v-for="tag in dashboardStats.mainTags" :key="tag.id" class="flex items-center gap-2">
               <div class="w-3 h-3 rounded-full shadow-inner" :style="{ backgroundColor: getTagColor(tag.id) }"></div>
               <span class="text-xs font-bold text-text-sec">{{ getTagName(tag.id) }} <span class="text-text-main ml-1">{{ tag.percentage.toFixed(0) }}%</span></span>
            </div>
         </div>
      </div>

      <!-- Detailed List -->
      <div class="space-y-4">
         <div class="mb-10 bg-bg-card border border-border-main rounded-3xl p-6 shadow-sm">
            <div class="flex items-start justify-between gap-6 mb-5">
              <div>
                <h3 class="text-sm font-bold text-text-sec mb-2 uppercase tracking-wider">未标副标签检测</h3>
                <div class="text-2xl font-black text-text-main">{{ formatMinutes(dashboardStats.missingSubTagTotalMinutes) }}</div>
              </div>
              <div class="text-right">
                <div class="text-xs font-bold text-text-sec mb-2">时间段数量</div>
                <div class="text-2xl font-black" :class="dashboardStats.missingSubTagCount > 0 ? 'text-[#FF9500]' : 'text-[#34C759]'">{{ dashboardStats.missingSubTagCount }}</div>
              </div>
            </div>

            <div v-if="dashboardStats.missingSubTags.length > 0" class="space-y-4">
              <div v-for="tag in dashboardStats.missingSubTags" :key="tag.id" class="border-t border-border-main/50 pt-4">
                <div class="flex items-center justify-between mb-3">
                  <div class="flex items-center gap-2">
                    <div class="w-3 h-3 rounded-full" :style="{ backgroundColor: getTagColor(tag.id) }"></div>
                    <span class="text-sm font-bold">{{ getTagName(tag.id) }}</span>
                  </div>
                  <div class="text-xs font-bold text-text-sec">
                    {{ formatMinutes(tag.total) }} · {{ tag.percentage.toFixed(1) }}%
                  </div>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                  <div v-for="ev in tag.events" :key="ev.id" class="bg-bg-input/60 rounded-xl px-3 py-2 text-xs flex items-center justify-between gap-3">
                    <span class="font-bold text-text-main whitespace-nowrap">{{ ev.date }}</span>
                    <span class="text-text-sec font-bold whitespace-nowrap">{{ logicalMinutesToTime(ev.start_minute) }} - {{ logicalMinutesToTime(ev.end_minute) }}</span>
                    <span class="text-text-sec truncate flex-1 text-right">{{ ev.content || '无备注' }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="border-t border-border-main/50 pt-5 text-sm font-bold text-text-sec">
              当前范围内所有事件都已标记副标签
            </div>
         </div>

         <h3 class="text-sm font-bold text-text-sec mb-4 uppercase tracking-wider">时间分布明细</h3>
         <div v-for="tag in dashboardStats.mainTags" :key="tag.id" class="bg-bg-card border border-border-main rounded-3xl p-5 shadow-sm transition-all hover:border-border-main/80 hover:shadow-md">
            <div class="flex justify-between items-center mb-1">
               <div class="flex items-center gap-3">
                  <div class="w-4 h-4 rounded-full shadow-inner" :style="{ backgroundColor: getTagColor(tag.id) }"></div>
                  <span class="font-bold text-lg">{{ getTagName(tag.id) }}</span>
               </div>
               <div class="text-right">
                  <div class="font-black text-xl">{{ formatMinutes(tag.total) }}</div>
               </div>
            </div>
            <div class="flex items-center gap-2 mb-4 pl-7 text-[11px] font-bold text-text-sec">
              <span class="bg-bg-input/50 px-2 py-1 rounded-md border border-border-main/30">占比 {{ tag.percentage.toFixed(1) }}%</span>
              <span class="bg-bg-input/50 px-2 py-1 rounded-md border border-border-main/30 text-[#007AFF]">日均 {{ formatMinutes(tag.dailyAverage) }}</span>
            </div>
            
            <!-- Sub-tags -->
            <div v-if="tag.subTags.length > 0" class="pl-7 space-y-3 pt-4 border-t border-border-main/50">
               <div v-for="sub in tag.subTags" :key="sub.id" class="flex justify-between items-center group gap-4">
                  <span class="text-xs text-text-main font-bold w-16 truncate">{{ getTagName(sub.id) }}</span>
                  <div class="flex items-center gap-3 flex-1 justify-end">
                     <div class="w-full max-w-50 h-1.5 bg-bg-input rounded-full overflow-hidden shadow-inner">
                        <div class="h-full rounded-full opacity-80" :style="{ width: sub.percentage + '%', backgroundColor: getTagColor(tag.id) }"></div>
                     </div>
                     <span class="text-[10px] font-bold text-text-sec whitespace-nowrap min-w-10 text-right">{{ formatMinutes(sub.total) }}</span>
                  </div>
               </div>
            </div>
         </div>
         
         <div v-if="dashboardStats.mainTags.length === 0" class="text-center py-20">
           <BarChart2 :size="48" class="mx-auto mb-4 text-text-sec opacity-20" />
           <div class="text-text-sec font-bold">该时间范围内没有记录数据</div>
         </div>
      </div>
    </div>
  </div>
</template>
