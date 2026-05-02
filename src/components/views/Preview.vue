<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Maximize2, RotateCcw, ZoomIn, ZoomOut } from "lucide-vue-next";
import { previewSrc } from "../../store";

const zoom = ref(1);
const offset = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const dragOrigin = ref({ x: 0, y: 0 });

const MIN_ZOOM = 1;
const MAX_ZOOM = 5;
const ZOOM_STEP = 0.25;

const imageStyle = computed(() => ({
  transform: `translate(${offset.value.x}px, ${offset.value.y}px) scale(${zoom.value})`,
  cursor: zoom.value > 1 ? (isDragging.value ? "grabbing" : "grab") : "default"
}));

const clampZoom = (value: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Number(value.toFixed(2))));

const resetView = () => {
  zoom.value = 1;
  offset.value = { x: 0, y: 0 };
  isDragging.value = false;
};

const setZoom = (nextZoom: number) => {
  zoom.value = clampZoom(nextZoom);
  if (zoom.value === 1) offset.value = { x: 0, y: 0 };
};

const handleWheel = (event: WheelEvent) => {
  if (!previewSrc.value) return;
  event.preventDefault();
  setZoom(zoom.value + (event.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP));
};

const startDrag = (event: MouseEvent) => {
  if (zoom.value <= 1) return;
  isDragging.value = true;
  dragStart.value = { x: event.clientX, y: event.clientY };
  dragOrigin.value = { ...offset.value };
};

const handleDrag = (event: MouseEvent) => {
  if (!isDragging.value) return;
  offset.value = {
    x: dragOrigin.value.x + event.clientX - dragStart.value.x,
    y: dragOrigin.value.y + event.clientY - dragStart.value.y
  };
};

const stopDrag = () => {
  isDragging.value = false;
};

watch(previewSrc, resetView);
</script>

<template>
  <div
    class="absolute inset-0 flex items-center justify-center p-12 bg-bg-input/30 overflow-hidden select-none"
    @wheel="handleWheel"
    @mousemove="handleDrag"
    @mouseup="stopDrag"
    @mouseleave="stopDrag"
  >
    <div v-if="previewSrc" class="absolute top-5 right-5 z-20 flex items-center gap-2 bg-bg-card/90 backdrop-blur-md border border-border-main rounded-2xl p-1.5 shadow-lg">
      <button @click="setZoom(zoom - ZOOM_STEP)" :disabled="zoom <= MIN_ZOOM" class="p-2 rounded-xl text-text-sec hover:text-text-main hover:bg-bg-input disabled:opacity-40 disabled:hover:bg-transparent">
        <ZoomOut :size="18" />
      </button>
      <div class="w-14 text-center text-xs font-black text-text-sec">{{ Math.round(zoom * 100) }}%</div>
      <button @click="setZoom(zoom + ZOOM_STEP)" :disabled="zoom >= MAX_ZOOM" class="p-2 rounded-xl text-text-sec hover:text-text-main hover:bg-bg-input disabled:opacity-40 disabled:hover:bg-transparent">
        <ZoomIn :size="18" />
      </button>
      <button @click="resetView" :disabled="zoom === 1 && offset.x === 0 && offset.y === 0" class="p-2 rounded-xl text-text-sec hover:text-text-main hover:bg-bg-input disabled:opacity-40 disabled:hover:bg-transparent">
        <RotateCcw :size="18" />
      </button>
    </div>

    <img
      v-if="previewSrc"
      :src="previewSrc"
      class="max-w-full max-h-full object-contain shadow-2xl border border-border-main/50 transition-transform duration-75"
      :style="imageStyle"
      draggable="false"
      @mousedown.prevent="startDrag"
      @dblclick="resetView"
    />
    <div v-else class="text-text-sec text-center">
      <Maximize2 :size="48" class="mx-auto mb-4 opacity-20" />
      <p>在时间轴上滑动或拖拽以记录</p>
    </div>
  </div>
</template>
