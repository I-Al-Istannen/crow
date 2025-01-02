<template>
  <div
    class="leading-none tracking-tight flex items-start justify-between p-3"
    :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
  >
    <span class="flex flex-col justify-center">
      <span>
        {{ task.info.revisionId }}
        <span class="ml-4 text-sm text-muted-foreground">{{ task.info.taskId }}</span>
      </span>
      <span v-if="tests && testStatistics" class="inline-grid grid-cols-5 justify-items-center gap-y-1 mt-2">
        <LucideTestTubeDiagonal class="inline w-[1em] h-[1em] text-blue-500" />
        <LucideCheck class="inline w-[1em] h-[1em] text-green-500" />
        <LucideX class="inline w-[1em] h-[1em] text-red-500" />
        <LucideClockAlert class="inline w-[1em] h-[1em] text-orange-500" />
        <LucideUnplug class="inline w-[1em] h-[1em] text-gray-500" />
        <span>{{ tests.length }}</span>
        <span>{{ testStatistics.finish }}</span>
        <span>{{ testStatistics.error }}</span>
        <span>{{ testStatistics.timeout }}</span>
        <span>{{ testStatistics.abort }}</span>
      </span>
    </span>
    <span class="text-sm text-muted-foreground flex flex-col justify-center items-end">
      <span>
        {{ formatTime(task.info.start) }}
      </span>
      <span>
        {{ formatDuration(task.info.start, task.info.end) }}
      </span>
    </span>
  </div>
</template>

<script setup lang="ts">
import {
  LucideCheck,
  LucideClockAlert,
  LucideTestTubeDiagonal,
  LucideUnplug,
  LucideX,
} from 'lucide-vue-next'
import { computed, toRefs } from 'vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'

const lucideIcon = 'inline w-[0.8em] h-[0.8em]'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
}>()
const { task } = toRefs(props)

const buildFailed = computed(() => task.value.type === 'BuildFailed')
const tests = computed(() => (task.value.type === 'RanTests' ? task.value.tests : undefined))

const testStatistics = computed(() => {
  if (tests.value === undefined) {
    return undefined
  }

  const finish = tests.value.filter((test) => test.output === 'Finished').length
  const abort = tests.value.filter((test) => test.output === 'Aborted').length
  const error = tests.value.filter((test) => test.output === 'Error').length
  const timeout = tests.value.filter((test) => test.output === 'Timeout').length

  return { finish, abort, error, timeout }
})

const formatTime = (date: Date) => {
  return new Intl.DateTimeFormat(undefined, {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  }).format(date)
}

const formatDuration = (start: Date, end: Date) => {
  const duration = end.getTime() - start.getTime()
  const seconds = Math.floor(duration / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) {
    return `${days}d ${hours % 24}h`
  } else if (hours > 0) {
    return `${hours}h ${minutes % 60}m`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  } else {
    return `${seconds}s`
  }
}
</script>
