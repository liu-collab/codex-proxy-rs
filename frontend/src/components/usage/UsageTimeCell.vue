<script setup lang="ts">
defineProps<{ value: string }>()
</script>

<template>
  <span class="block truncate font-mono text-cp-sm leading-none font-bold text-cp-text-secondary" :title="value">
    {{ value || '—' }}
  </span>
</template>
