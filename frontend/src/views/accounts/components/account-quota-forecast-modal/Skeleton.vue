<script setup lang="ts">
import { ZSkeleton } from '@codex-proxy/ui'
</script>

<template>
  <div role="status" aria-busy="true">
    <span class="sr-only">正在加载额度预测</span>
    <div aria-hidden="true">
      <div class="flex flex-col gap-5 rounded-cp-card bg-cp-fill-tertiary/70 p-4 [html[data-theme=light]_&]:bg-cp-fill-quaternary/70">
        <div>
          <div class="flex items-center justify-between gap-3">
            <ZSkeleton shape="text" class="w-16" />
            <ZSkeleton class="h-6 w-28" />
          </div>
          <ZSkeleton class="mt-2 h-8 w-28" />
          <ZSkeleton class="mt-3 h-3 w-full rounded-full" />
          <div class="mt-2 flex justify-between">
            <ZSkeleton v-for="tick in 5" :key="tick" shape="text" class="h-2.5 w-5" />
          </div>
        </div>

        <div class="grid gap-2">
          <div class="mb-1 flex justify-between gap-3">
            <ZSkeleton shape="text" class="w-20" />
            <ZSkeleton shape="text" class="w-20" />
          </div>
          <div v-for="metric in 2" :key="metric" class="rounded-cp bg-cp-bg-container px-3 py-3">
            <ZSkeleton shape="text" class="w-20" />
            <div class="mt-3 grid grid-cols-[1fr_20px_1.2fr] items-center gap-2">
              <ZSkeleton shape="text" class="w-14" />
              <ZSkeleton class="size-3" />
              <ZSkeleton class="h-6 w-24 justify-self-end" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
