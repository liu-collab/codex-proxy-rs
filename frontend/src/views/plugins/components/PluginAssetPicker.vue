<script setup lang="ts">
import type { PluginReleaseAsset } from '@/api'

import { ZPopover, ZScrollbar, ZTag } from '@codex-proxy/ui'
import { ChevronDown, FileArchive } from '@lucide/vue'
import { useElementSize, useEventListener } from '@vueuse/core'
import { computed, nextTick, shallowRef, useId, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatPluginFileSize } from '../utils/model'

const props = defineProps<{ assets: PluginReleaseAsset[], disabled?: boolean }>()

const { t } = useI18n({ useScope: 'global' })

const selected = defineModel<string>({ required: true })
const open = shallowRef(false)
const trigger = useTemplateRef<HTMLButtonElement>('trigger')
const panel = useTemplateRef<HTMLDivElement>('panel')
const panelId = useId()
const { width } = useElementSize(trigger, undefined, { box: 'border-box' })
const selectedAsset = computed(() => props.assets.find(asset => asset.name === selected.value))

async function close() {
  open.value = false
  await nextTick()
  trigger.value?.focus()
}

function select(name: string) {
  if (props.disabled)
    return
  selected.value = name
  void close()
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape' || !open.value)
    return
  event.preventDefault()
  event.stopPropagation()
  void close()
}

useEventListener(panel, 'keydown', handleKeydown)
useEventListener(panel, 'keydown', (event) => {
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key))
    return
  const options = [
    ...(panel.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? []),
  ]
  if (!options.length)
    return
  event.preventDefault()
  event.stopPropagation()
  const current = options.indexOf(event.target as HTMLButtonElement)
  const next = event.key === 'Home'
    ? 0
    : event.key === 'End'
      ? options.length - 1
      : (current + (event.key === 'ArrowDown'
          ? 1
          : -1) + options.length) % options.length
  options[next]?.focus()
})
watch(open, async (value) => {
  if (!value)
    return
  await nextTick()
  if (open.value)
    (panel.value?.querySelector<HTMLButtonElement>('[aria-selected="true"]') ?? panel.value?.querySelector<HTMLButtonElement>('button:not(:disabled)'))?.focus()
})
watch(() => props.disabled, (value) => {
  if (value)
    open.value = false
})
</script>

<template>
  <ZPopover v-model="open" placement="bottom-start" :disabled="disabled" class="w-full min-w-0">
    <template #reference>
      <button
        ref="trigger"
        type="button"
        :disabled="disabled"
        :title="selectedAsset?.name"
        :aria-label="selectedAsset ? t('plugins.assets.pluginPackage', { name: selectedAsset.name }) : t('plugins.assets.selectPluginPackage')"
        aria-haspopup="listbox"
        :aria-expanded="open"
        :aria-controls="open ? panelId : undefined"
        class="flex min-h-z-control w-full min-w-0 items-center gap-3 rounded-z border-0 px-3.5 py-2 text-left text-z shadow-z-input outline-none transition-[background-color,box-shadow] duration-160 motion-reduce:transition-none"
        :class="disabled
          ? 'cursor-not-allowed bg-z-bg-container-disabled text-z-text-disabled shadow-none'
          : open
            ? 'cursor-pointer bg-(--z-input-active-bg) text-z-text shadow-z-input-active'
            : 'cursor-pointer bg-(--z-input-bg) text-z-text hover:bg-(--z-input-hover-bg) hover:shadow-z-input-hover focus-visible:bg-(--z-input-active-bg) focus-visible:shadow-z-input-active'"
        @keydown="handleKeydown"
        @keydown.down.prevent="open = !disabled"
      >
        <FileArchive v-if="selectedAsset" class="size-4 shrink-0 text-z-text-tertiary" aria-hidden="true" />
        <span class="min-w-0 flex-1 truncate text-z-sm" :class="{ 'text-z-text-quaternary': !selectedAsset }">
          {{ selectedAsset?.name ?? t('plugins.assets.selectPluginPackage') }}
        </span>
        <ChevronDown class="size-4 shrink-0 text-z-text-tertiary transition-transform motion-reduce:transition-none" :class="{ 'rotate-180': open }" aria-hidden="true" />
      </button>
    </template>
    <div :id="panelId" ref="panel" role="listbox" :aria-label="t('plugins.assets.packageLabel')" aria-required="true" :style="{ width: `${width}px` }" class="max-w-[calc(100vw-2rem)] p-1.5">
      <ZScrollbar max-height="min(18rem, 45dvh)">
        <div class="grid gap-0.5 p-0.5">
          <button
            v-for="asset in assets"
            :key="asset.name"
            type="button"
            role="option"
            tabindex="-1"
            :title="asset.name"
            :aria-selected="selected === asset.name"
            :disabled="disabled"
            class="flex w-full min-w-0 items-center gap-3 rounded-z-sm border-0 px-3 py-2.5 text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-z-control-outline focus-visible:ring-inset"
            :class="disabled ? 'cursor-not-allowed opacity-55' : selected === asset.name ? 'cursor-pointer bg-z-control-item-bg-active hover:bg-z-control-item-bg-active-hover' : 'cursor-pointer bg-transparent hover:bg-z-fill-tertiary'"
            @click="select(asset.name)"
          >
            <span class="min-w-0 flex-1 truncate text-z-sm" :class="selected === asset.name ? 'text-z-primary-text' : 'text-z-text'">{{ asset.name }}</span>
            <ZTag class="shrink-0 tabular-nums">
              {{ formatPluginFileSize(asset.size) }}
            </ZTag>
          </button>
        </div>
      </ZScrollbar>
    </div>
  </ZPopover>
</template>
