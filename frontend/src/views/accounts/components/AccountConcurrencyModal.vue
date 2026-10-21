<script setup lang="ts">
import type { Account } from '@/api'
import { ZDialog } from '@codex-proxy/ui'
import { toRef } from 'vue'
import { useAccountConcurrency } from '../composables/useAccountConcurrency'
import AccountConcurrencyTable from './AccountConcurrencyTable.vue'

const props = defineProps<{ account: Account | null }>()
const open = defineModel<boolean>({ required: true })
const { data, entries, loading, error } = useAccountConcurrency(toRef(props, 'account'), open)
</script>

<template>
  <ZDialog v-model="open" title="并发明细" description="当前正在处理的请求" width="72rem">
    <AccountConcurrencyTable
      :entries="entries"
      :loading="loading && !data" :aria-busy="loading"
      :empty-text="error ? '暂时无法读取并发明细' : '当前没有并发占用'"
    />
  </ZDialog>
</template>
