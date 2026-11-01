<script setup lang="ts">
defineProps<{ value: string }>()
</script>

<template>
  <span class="block truncate font-mono text-z-sm leading-none font-bold text-z-text-secondary" :title="value">
    {{ value || '—' }}
  </span>
</template>
