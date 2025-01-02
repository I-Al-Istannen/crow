<template>
  <router-link :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }" class="block">
    <div
      class="leading-none tracking-tight flex items-start justify-between p-2 hover:bg-accent hover:text-accent-foreground"
      :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
    >
      <span class="flex flex-col justify-center">
        <span class="mb-1 font-medium">{{ task.info.revisionId }}</span>
        <span v-if="tests && testStatistics" class="text-sm space-x-2">
          <span>{{ tests.length }} tests</span>
          <span v-if="testStatistics.finish > 0" class="text-green-500">
            {{ testStatistics.finish }} finished
          </span>
          <span v-if="testStatistics.error > 0" class="text-red-500">
            {{ testStatistics.error }} errors
          </span>
          <span v-if="testStatistics.timeout > 0" class="text-orange-500">
            {{ testStatistics.timeout }} timeouts
          </span>
          <span v-if="testStatistics.abort > 0" class="text-gray-500">
            {{ testStatistics.abort }} aborted
          </span>
        </span>
        <span v-else class="text-red-500"> build failed </span>
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
  </router-link>
</template>

<script setup lang="ts">
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
