<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  CalendarDays,
  Check,
  ChevronRight,
  Clock3,
  EyeOff,
  ListFilter,
  Plus,
  RotateCcw,
  Trash2,
  X,
} from "lucide-vue-next";
import {
  currentDate,
  getSubTags,
  getTagColor,
  getTagName,
  loadPlanTasks,
  mainTags,
  parseISODate,
  planTasks,
  showToast,
  todayLogicalDate,
  toISODate,
} from "../../store";
import type { PlanTask } from "../../types";

const emit = defineEmits<{ selectDate: [date: string] }>();

const DAY_WIDTH = 68;
const DAYS_BEFORE = 7;
const DAYS_AFTER = 14;

const onlyFocusDate = ref(false);
const hideCompleted = ref(true);
const editingTask = ref<PlanTask | null>(null);
const suppressTaskClick = ref<number | null>(null);

const logicalToday = computed(() => todayLogicalDate.value);

const addDays = (dateString: string, days: number) => {
  const date = parseISODate(dateString);
  date.setDate(date.getDate() + days);
  return toISODate(date);
};

const dayDifference = (start: string, end: string) => {
  return Math.round((parseISODate(end).getTime() - parseISODate(start).getTime()) / 86400000);
};

const rangeStart = computed(() => addDays(currentDate.value, -DAYS_BEFORE));
const rangeEnd = computed(() => addDays(currentDate.value, DAYS_AFTER));
const rangeDays = computed(() =>
  Array.from({ length: DAYS_BEFORE + DAYS_AFTER + 1 }, (_, index) => addDays(rangeStart.value, index)),
);
const gridTemplate = computed(() => `260px repeat(${rangeDays.value.length}, ${DAY_WIDTH}px)`);
const focusIsToday = computed(() => currentDate.value === logicalToday.value);

const isActiveOn = (task: PlanTask, date: string) => task.start_date <= date && task.end_date >= date;
const isOverdue = (task: PlanTask) => !task.is_completed && task.end_date < logicalToday.value;

const taskStatus = (task: PlanTask) => {
  if (task.is_completed) return "已完成";
  if (task.end_date < logicalToday.value) return "已逾期";
  if (task.start_date > logicalToday.value) return "未开始";
  return "进行中";
};

const taskStatusClass = (task: PlanTask) => {
  const status = taskStatus(task);
  if (status === "已逾期") return "text-[#FF3B30] bg-[#FF3B30]/10";
  if (status === "进行中") return "text-[#007AFF] bg-[#007AFF]/10";
  if (status === "已完成") return "text-[#34C759] bg-[#34C759]/10";
  return "text-text-sec bg-bg-input";
};

const statusOrder = (task: PlanTask) => {
  if (isOverdue(task)) return 0;
  if (!task.is_completed && isActiveOn(task, logicalToday.value)) return 1;
  if (!task.is_completed) return 2;
  return 3;
};

const visibleTasks = computed(() => {
  return planTasks.value
    .filter(task => !hideCompleted.value || !task.is_completed)
    .filter(task => {
      if (onlyFocusDate.value) {
        return isActiveOn(task, currentDate.value) || (focusIsToday.value && isOverdue(task));
      }
      const intersectsRange = task.start_date <= rangeEnd.value && task.end_date >= rangeStart.value;
      return intersectsRange || isOverdue(task);
    })
    .slice()
    .sort((a, b) =>
      statusOrder(a) - statusOrder(b) ||
      a.end_date.localeCompare(b.end_date) ||
      a.start_date.localeCompare(b.start_date) ||
      a.id - b.id,
    );
});

const summary = computed(() => ({
  active: planTasks.value.filter(task => !task.is_completed && isActiveOn(task, logicalToday.value)).length,
  overdue: planTasks.value.filter(isOverdue).length,
  dueToday: planTasks.value.filter(task => !task.is_completed && task.end_date === logicalToday.value).length,
  focus: planTasks.value.filter(task => !task.is_completed && isActiveOn(task, currentDate.value)).length,
}));

const createTaskDraft = (): PlanTask => ({
  id: 0,
  title: "",
  start_date: currentDate.value,
  end_date: currentDate.value,
  main_tag_id: null,
  sub_tag_id: null,
  notes: "",
  is_completed: false,
  completed_at: null,
});

const openNewTask = () => {
  editingTask.value = createTaskDraft();
};

const openTask = (task: PlanTask) => {
  if (suppressTaskClick.value === task.id) return;
  editingTask.value = { ...task };
};

const handleMainTagChange = () => {
  if (!editingTask.value) return;
  const validSubTag = getSubTags(editingTask.value.main_tag_id ?? 0)
    .some(tag => tag.id === editingTask.value?.sub_tag_id);
  if (!validSubTag) editingTask.value.sub_tag_id = null;
};

const saveTask = async () => {
  if (!editingTask.value) return;
  if (!editingTask.value.title.trim()) {
    showToast("请输入任务名称", "error");
    return;
  }
  if (editingTask.value.end_date < editingTask.value.start_date) {
    showToast("结束日期不能早于开始日期", "error");
    return;
  }

  const wasEditing = editingTask.value.id > 0;
  try {
    await invoke("save_plan_task", { task: editingTask.value });
    editingTask.value = null;
    await loadPlanTasks();
    showToast(wasEditing ? "任务已更新" : "任务已创建");
  } catch (error) {
    showToast("保存失败: " + error, "error");
  }
};

const deleteTask = async (task: PlanTask) => {
  if (!window.confirm(`确定删除任务“${task.title}”吗？`)) return;
  try {
    await invoke("delete_plan_task", { id: task.id });
    editingTask.value = null;
    await loadPlanTasks();
    showToast("任务已删除");
  } catch (error) {
    showToast("删除失败: " + error, "error");
  }
};

const toggleTask = async (task: PlanTask) => {
  try {
    await invoke("toggle_plan_task", { id: task.id, isCompleted: !task.is_completed });
    await loadPlanTasks();
    showToast(task.is_completed ? "任务已恢复" : "任务已完成");
  } catch (error) {
    showToast("更新失败: " + error, "error");
  }
};

const getBarPlacement = (task: PlanTask) => {
  if (task.end_date < rangeStart.value || task.start_date > rangeEnd.value) return null;
  const startIndex = Math.max(0, dayDifference(rangeStart.value, task.start_date));
  const endIndex = Math.min(rangeDays.value.length - 1, dayDifference(rangeStart.value, task.end_date));
  return {
    gridColumn: `${startIndex + 2} / span ${endIndex - startIndex + 1}`,
  };
};

const taskBarStyle = (task: PlanTask) => ({
  ...getBarPlacement(task),
  gridRow: "1",
  backgroundColor: isOverdue(task) ? "#FF3B30" : getTagColor(task.main_tag_id),
});

type DragMode = "move" | "start" | "end";
type DragState = {
  taskId: number;
  mode: DragMode;
  originX: number;
  originalStart: string;
  originalEnd: string;
  moved: boolean;
};

let dragState: DragState | null = null;

const startTaskDrag = (event: MouseEvent, task: PlanTask, mode: DragMode) => {
  dragState = {
    taskId: task.id,
    mode,
    originX: event.clientX,
    originalStart: task.start_date,
    originalEnd: task.end_date,
    moved: false,
  };
  window.addEventListener("mousemove", handleTaskDrag);
  window.addEventListener("mouseup", finishTaskDrag);
};

const handleTaskDrag = (event: MouseEvent) => {
  if (!dragState) return;
  const deltaDays = Math.round((event.clientX - dragState.originX) / DAY_WIDTH);
  const task = planTasks.value.find(item => item.id === dragState?.taskId);
  if (!task) return;

  dragState.moved = dragState.moved || deltaDays !== 0;
  if (dragState.mode === "move") {
    task.start_date = addDays(dragState.originalStart, deltaDays);
    task.end_date = addDays(dragState.originalEnd, deltaDays);
  } else if (dragState.mode === "start") {
    const nextStart = addDays(dragState.originalStart, deltaDays);
    task.start_date = nextStart <= dragState.originalEnd ? nextStart : dragState.originalEnd;
  } else {
    const nextEnd = addDays(dragState.originalEnd, deltaDays);
    task.end_date = nextEnd >= dragState.originalStart ? nextEnd : dragState.originalStart;
  }
};

const finishTaskDrag = async () => {
  window.removeEventListener("mousemove", handleTaskDrag);
  window.removeEventListener("mouseup", finishTaskDrag);
  const state = dragState;
  dragState = null;
  if (!state?.moved) return;

  suppressTaskClick.value = state.taskId;
  window.setTimeout(() => {
    suppressTaskClick.value = null;
  }, 0);

  const task = planTasks.value.find(item => item.id === state.taskId);
  if (!task) return;
  try {
    await invoke("save_plan_task", { task: { ...task } });
    showToast("任务日期已更新");
  } catch (error) {
    await loadPlanTasks();
    showToast("日期更新失败: " + error, "error");
  }
};

const isWeekend = (date: string) => {
  const day = parseISODate(date).getDay();
  return day === 0 || day === 6;
};

const formatDay = (date: string) => {
  const parsed = parseISODate(date);
  return `${parsed.getMonth() + 1}月${parsed.getDate()}日`;
};

const weekdayName = (date: string) => "日一二三四五六"[parseISODate(date).getDay()];

onMounted(loadPlanTasks);
onUnmounted(() => {
  window.removeEventListener("mousemove", handleTaskDrag);
  window.removeEventListener("mouseup", finishTaskDrag);
});
</script>

<template>
  <div class="absolute inset-0 flex flex-col bg-bg-main overflow-hidden">
    <div class="px-6 py-5 border-b border-border-main/70 bg-bg-card/60">
      <div class="flex items-start justify-between gap-6">
        <div>
          <div class="flex items-center gap-2 mb-2">
            <span class="text-sm font-black text-text-main">{{ focusIsToday ? "今天" : currentDate }}</span>
            <span class="text-[10px] font-bold text-text-sec bg-bg-input px-2 py-1 rounded-lg">03:00 起算</span>
          </div>
          <div v-if="focusIsToday" class="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs font-bold">
            <span class="text-[#007AFF]">进行中 {{ summary.active }} 项</span>
            <span :class="summary.overdue ? 'text-[#FF3B30]' : 'text-text-sec'">已逾期 {{ summary.overdue }} 项</span>
            <span class="text-text-sec">今天截止 {{ summary.dueToday }} 项</span>
          </div>
          <div v-else class="text-xs font-bold text-text-sec">该日有 {{ summary.focus }} 项未完成任务</div>
        </div>

        <div class="flex items-center gap-2 shrink-0">
          <button
            v-if="!focusIsToday"
            @click="emit('selectDate', logicalToday)"
            class="h-10 px-3 rounded-xl border border-border-main bg-bg-card text-xs font-bold text-text-sec hover:text-[#007AFF] hover:border-[#007AFF]/30 flex items-center gap-2"
          >
            <RotateCcw :size="15" /> 回到今天
          </button>
          <button
            @click="onlyFocusDate = !onlyFocusDate"
            class="h-10 px-3 rounded-xl border text-xs font-bold flex items-center gap-2 transition-colors"
            :class="onlyFocusDate ? 'border-[#007AFF]/30 bg-[#007AFF]/10 text-[#007AFF]' : 'border-border-main bg-bg-card text-text-sec hover:text-text-main'"
          >
            <ListFilter :size="15" /> 仅看该日
          </button>
          <button
            @click="hideCompleted = !hideCompleted"
            class="h-10 px-3 rounded-xl border text-xs font-bold flex items-center gap-2 transition-colors"
            :class="hideCompleted ? 'border-[#007AFF]/30 bg-[#007AFF]/10 text-[#007AFF]' : 'border-border-main bg-bg-card text-text-sec hover:text-text-main'"
          >
            <EyeOff :size="15" /> 隐藏已完成
          </button>
          <button @click="openNewTask" class="h-10 px-4 rounded-xl bg-[#007AFF] text-white text-xs font-bold shadow-lg shadow-[#007AFF]/20 flex items-center gap-2 hover:brightness-110">
            <Plus :size="16" /> 新建任务
          </button>
        </div>
      </div>
    </div>

    <div class="flex-1 overflow-auto bg-bg-card/30">
      <div class="min-w-max">
        <div class="grid sticky top-0 z-30 border-b border-border-main bg-bg-card/95 backdrop-blur-md" :style="{ gridTemplateColumns: gridTemplate }">
          <div class="sticky left-0 z-40 h-16 px-5 flex items-center bg-bg-card border-r border-border-main text-[11px] font-black text-text-sec uppercase tracking-wider">
            任务
          </div>
          <button
            v-for="date in rangeDays"
            :key="date"
            @click="emit('selectDate', date)"
            class="h-16 border-r border-border-main/60 flex flex-col items-center justify-center transition-colors relative"
            :class="[
              isWeekend(date) ? 'bg-bg-input/35' : '',
              date === currentDate ? 'bg-[#007AFF]/10 text-[#007AFF]' : 'hover:bg-bg-input/60',
              date === logicalToday ? 'border-l-2 border-l-[#FF3B30]' : '',
            ]"
          >
            <span class="text-[10px] font-bold opacity-65 mb-1">周{{ weekdayName(date) }}</span>
            <span class="text-xs font-black">{{ formatDay(date) }}</span>
            <span v-if="date === logicalToday" class="absolute bottom-0 left-1/2 -translate-x-1/2 w-1.5 h-1.5 rounded-full bg-[#FF3B30]"></span>
          </button>
        </div>

        <div v-if="visibleTasks.length">
          <div
            v-for="task in visibleTasks"
            :key="task.id"
            class="grid h-18 border-b border-border-main/60 group"
            :style="{ gridTemplateColumns: gridTemplate }"
          >
            <div class="sticky left-0 z-20 bg-bg-card border-r border-border-main px-4 flex items-center gap-3 min-w-0 group-hover:bg-bg-input/35">
              <button
                @click.stop="toggleTask(task)"
                class="w-6 h-6 rounded-full shrink-0 flex items-center justify-center transition-all"
                :class="task.is_completed ? 'bg-[#34C759] text-white' : 'border-2 border-border-main text-transparent hover:border-[#007AFF] hover:text-[#007AFF]/40'"
                :title="task.is_completed ? '恢复任务' : '完成任务'"
              >
                <Check :size="14" />
              </button>
              <button @click="openTask(task)" class="flex-1 min-w-0 text-left py-2">
                <div class="flex items-center gap-2 mb-1">
                  <span class="font-bold text-sm truncate" :class="task.is_completed ? 'line-through text-text-sec' : 'text-text-main'">{{ task.title }}</span>
                  <ChevronRight :size="13" class="shrink-0 text-text-sec opacity-0 group-hover:opacity-100" />
                </div>
                <div class="flex items-center gap-2 min-w-0">
                  <span class="text-[9px] font-black px-1.5 py-0.5 rounded-md shrink-0" :class="taskStatusClass(task)">{{ taskStatus(task) }}</span>
                  <span v-if="task.main_tag_id" class="text-[10px] font-bold text-text-sec truncate">
                    {{ getTagName(task.main_tag_id) }}<template v-if="task.sub_tag_id"> / {{ getTagName(task.sub_tag_id) }}</template>
                  </span>
                  <span v-else class="text-[10px] font-bold text-text-sec">无标签</span>
                </div>
              </button>
            </div>

            <div
              v-for="date in rangeDays"
              :key="`${task.id}-${date}`"
              class="border-r border-border-main/40 relative"
              :class="[
                isWeekend(date) ? 'bg-bg-input/25' : '',
                date === currentDate ? 'bg-[#007AFF]/6' : '',
                date === logicalToday ? 'border-l-2 border-l-[#FF3B30]/80' : '',
              ]"
            ></div>

            <button
              v-if="getBarPlacement(task)"
              @click="openTask(task)"
              @mousedown.stop.prevent="startTaskDrag($event, task, 'move')"
              class="z-10 self-center h-9 mx-1 rounded-xl shadow-sm text-white px-3 flex items-center min-w-0 cursor-grab active:cursor-grabbing hover:brightness-110 transition-[filter,opacity] relative overflow-hidden"
              :class="task.is_completed ? 'opacity-45' : ''"
              :style="taskBarStyle(task)"
              :title="`${task.title} · ${task.start_date} 至 ${task.end_date}`"
            >
              <span
                @mousedown.stop.prevent="startTaskDrag($event, task, 'start')"
                class="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-white/35"
              ></span>
              <span class="text-[11px] font-black truncate drop-shadow-sm">{{ task.title }}</span>
              <span
                @mousedown.stop.prevent="startTaskDrag($event, task, 'end')"
                class="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-white/35"
              ></span>
            </button>

            <div
              v-else-if="isOverdue(task)"
              class="z-10 self-center h-9 mx-1 px-3 rounded-xl bg-[#FF3B30]/12 border border-[#FF3B30]/25 text-[#FF3B30] flex items-center gap-2"
              style="grid-column: 2 / span 3; grid-row: 1"
            >
              <Clock3 :size="13" />
              <span class="text-[10px] font-black truncate">已于 {{ task.end_date }} 截止</span>
            </div>
          </div>
        </div>

        <div v-else class="h-80 sticky left-0 flex flex-col items-center justify-center text-text-sec" style="width: min(100vw, 900px)">
          <CalendarDays :size="52" class="opacity-20 mb-4" />
          <div class="text-sm font-bold mb-1">{{ planTasks.length ? "当前范围内没有符合条件的任务" : "还没有计划任务" }}</div>
          <div class="text-xs opacity-70 mb-5">从一个有明确起止日期的任务开始</div>
          <button @click="openNewTask" class="px-4 py-2.5 rounded-xl bg-[#007AFF] text-white text-xs font-bold flex items-center gap-2">
            <Plus :size="15" /> 新建任务
          </button>
        </div>
      </div>
    </div>

    <div v-if="editingTask" class="fixed inset-0 z-120 bg-black/40 backdrop-blur-sm flex items-center justify-center p-6" @click.self="editingTask = null">
      <div class="bg-bg-card rounded-4xl shadow-2xl w-full max-w-xl overflow-hidden border border-border-main">
        <div class="p-7 border-b border-border-main/60 flex items-center justify-between">
          <div>
            <h2 class="text-xl font-black">{{ editingTask.id ? "编辑任务" : "新建任务" }}</h2>
            <p class="text-xs font-bold text-text-sec mt-1">任务日期按每天 03:00 开始计算</p>
          </div>
          <button @click="editingTask = null" class="p-2 rounded-xl text-text-sec hover:bg-bg-input hover:text-text-main"><X :size="22" /></button>
        </div>

        <div class="p-7 space-y-6">
          <div>
            <label class="text-[10px] font-bold text-text-sec block mb-2">任务名称</label>
            <input v-model="editingTask.title" autofocus maxlength="120" placeholder="需要完成什么？" class="w-full bg-bg-input border border-transparent rounded-2xl px-4 py-3.5 text-sm font-bold outline-none focus:bg-bg-card focus:border-[#007AFF]/40" @keydown.ctrl.enter="saveTask" />
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="text-[10px] font-bold text-text-sec block mb-2">开始日期</label>
              <input v-model="editingTask.start_date" type="date" class="w-full bg-bg-input border border-transparent rounded-2xl px-4 py-3 text-sm font-bold outline-none focus:bg-bg-card focus:border-[#007AFF]/40" />
            </div>
            <div>
              <label class="text-[10px] font-bold text-text-sec block mb-2">结束日期</label>
              <input v-model="editingTask.end_date" type="date" class="w-full bg-bg-input border border-transparent rounded-2xl px-4 py-3 text-sm font-bold outline-none focus:bg-bg-card focus:border-[#007AFF]/40" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="text-[10px] font-bold text-text-sec block mb-2">主标签（可选）</label>
              <select v-model="editingTask.main_tag_id" @change="handleMainTagChange" class="w-full bg-bg-input border border-transparent rounded-2xl px-4 py-3 text-sm font-bold outline-none focus:bg-bg-card focus:border-[#007AFF]/40">
                <option :value="null">无标签</option>
                <option v-for="tag in mainTags" :key="tag.id" :value="tag.id">{{ tag.name }}</option>
              </select>
            </div>
            <div>
              <label class="text-[10px] font-bold text-text-sec block mb-2">副标签（可选）</label>
              <select v-model="editingTask.sub_tag_id" :disabled="!editingTask.main_tag_id || !getSubTags(editingTask.main_tag_id).length" class="w-full bg-bg-input border border-transparent rounded-2xl px-4 py-3 text-sm font-bold outline-none focus:bg-bg-card focus:border-[#007AFF]/40 disabled:opacity-45">
                <option :value="null">无副标签</option>
                <option v-for="tag in getSubTags(editingTask.main_tag_id || 0)" :key="tag.id" :value="tag.id">{{ tag.name }}</option>
              </select>
            </div>
          </div>

          <div>
            <label class="text-[10px] font-bold text-text-sec block mb-2">备注（可选）</label>
            <textarea v-model="editingTask.notes" rows="4" placeholder="补充任务背景、交付标准或相关信息…" class="w-full resize-none bg-bg-input border border-transparent rounded-2xl px-4 py-3 text-sm outline-none focus:bg-bg-card focus:border-[#007AFF]/40" @keydown.ctrl.enter="saveTask"></textarea>
          </div>

          <div class="flex items-center gap-3 pt-1">
            <button v-if="editingTask.id" @click="deleteTask(editingTask)" class="px-4 py-3.5 rounded-2xl text-[#FF3B30] text-sm font-bold hover:bg-[#FF3B30]/10 flex items-center gap-2">
              <Trash2 :size="17" /> 删除
            </button>
            <div class="flex-1"></div>
            <button @click="editingTask = null" class="px-5 py-3.5 rounded-2xl bg-bg-input text-text-sec text-sm font-bold hover:text-text-main">取消</button>
            <button @click="saveTask" class="px-6 py-3.5 rounded-2xl bg-[#007AFF] text-white text-sm font-bold shadow-lg shadow-[#007AFF]/20">保存任务</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
