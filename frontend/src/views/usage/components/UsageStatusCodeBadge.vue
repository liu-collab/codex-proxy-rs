<script setup lang="ts">
defineProps<{
  statusCode?: number | null
}>()

function statusClass(statusCode?: number | null) {
  if (statusCode === undefined || statusCode === null) {
    return 'bg-z-fill-quaternary text-z-text-secondary'
  }

  if (statusCode >= 200 && statusCode < 300) {
    return 'bg-z-success-container text-z-success-on-container'
  }

  if (statusCode >= 300 && statusCode < 400) {
    return 'bg-z-warning-container text-z-warning-on-container'
  }

  return 'bg-z-error-container text-z-error-on-container'
}
</script>

<template>
  <span
    class="inline-flex h-6 min-w-12 items-center justify-center rounded-full px-2 font-mono text-z-sm leading-none font-bold"
    :class="statusClass(statusCode)"
  >
    {{ statusCode ?? '—' }}
  </span>
</template>
