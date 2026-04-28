<script setup lang="ts">
import { Maximize2 } from "lucide-vue-next";
import { previewSrc } from "../../store";
</script>

<template>
  <div class="absolute inset-0 flex items-center justify-center p-12 bg-bg-input/30">
    <img v-if="previewSrc" :src="previewSrc" class="max-w-full max-h-full object-contain rounded-3xl shadow-2xl border border-border-main/50" />
    <div v-else class="text-text-sec text-center">
      <Maximize2 :size="48" class="mx-auto mb-4 opacity-20" />
      <p>在时间轴上滑动或拖拽以记录</p>
    </div>
  </div>
</template>
