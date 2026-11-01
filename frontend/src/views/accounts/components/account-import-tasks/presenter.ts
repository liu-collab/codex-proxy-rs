import type { AccountImportTask, AccountImportTaskItem, ImportItemStatus } from '@/api'
import { computed } from 'vue'
import { t } from '@/i18n'

export const itemStates = computed<Record<ImportItemStatus, { label: string, color: string, fill: string }>>(() => ({
  pending: {
    label: t('common.status.pending'),
    color: 'bg-z-fill-alter text-z-text-secondary',
    fill: 'bg-z-fill-secondary',
  },
  running: {
    label: t('accounts.importTasks.processing'),
    color: 'bg-z-primary-container text-z-primary-on-container',
    fill: 'bg-z-primary',
  },
  succeeded: {
    label: t('accounts.importTasks.alreadyInStock'),
    color: 'bg-z-success-container text-z-success-on-container',
    fill: 'bg-z-success',
  },
  failed: {
    label: t('common.status.failed'),
    color: 'bg-z-error-container text-z-error-on-container',
    fill: 'bg-z-error',
  },
  unknown: {
    label: t('accounts.importTasks.toBeVerified'),
    color: 'bg-z-warning-container text-z-warning-on-container',
    fill: 'bg-z-warning',
  },
  skipped: {
    label: t('accounts.importTasks.notExecuted'),
    color: 'bg-z-fill-tertiary text-z-text-secondary',
    fill: 'bg-z-text-quaternary',
  },
}))

export const outcomeOrder: ImportItemStatus[] = [
  'succeeded',
  'failed',
  'unknown',
  'running',
  'pending',
  'skipped',
]

export function taskLabel(task: AccountImportTask) {
  if (!task.finishedAt) {
    return task.stopRequested
      ? t('accounts.importTasks.stopping')
      : task.counts.running > 0
        ? t('accounts.importTasks.importing')
        : t('accounts.importTasks.queuing')
  }
  if (task.stopRequested)
    return t('accounts.importTasks.stopped')
  return task.counts.failed + task.counts.unknown > 0
    ? t('accounts.importTasks.endedPendingItems')
    : t('accounts.importTasks.allDone')
}

export function processed(task: AccountImportTask) {
  return task.total - task.counts.pending - task.counts.running
}

export function itemDescription(item: AccountImportTaskItem) {
  if (item.message)
    return item.message.replace(/[。.]\s*$/u, '')
  switch (item.status) {
    case 'succeeded': return t('accounts.importTasks.savedAccountSavedAccounts', { count: item.accountIds.length })
    case 'running': return t('accounts.importTasks.verifying')
    case 'pending': return t('accounts.importTasks.waitingForExecution')
    case 'skipped': return t('accounts.importTasks.thisArticleIsNotImplemented')
    case 'failed': return t('accounts.importTasks.entryFailed')
    case 'unknown': return t('accounts.importTasks.checkAccountList')
  }
}
