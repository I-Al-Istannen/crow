<template>
  <router-link :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }" class="block">
    <div
      class="leading-none tracking-tight flex items-start justify-between p-2 hover:bg-accent hover:text-accent-foreground"
      :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
    >
      <span class="flex flex-col justify-center">
        <span class="mb-1">
          <span class="text-muted-foreground">{{ task.info.revisionId.substring(0, 8) }}: </span>
          <span class="font-medium">{{ task.info.commitMessage }}</span>
        </span>
        <TaskQuickOverview class="text-sm" :task="task" />
      </span>
      <span class="text-sm text-muted-foreground flex flex-col justify-center items-end">
        <span>
          {{ formatTime(task.info.start) }}
        </span>
        <span>
          {{ formatDurationBetween(task.info.start, task.info.end) }}
        </span>
      </span>
    </div>
  </router-link>
</template>

<script setup lang="ts">
import { formatDurationBetween, formatTime } from '../lib/utils.ts'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import TaskQuickOverview from '@/components/TaskQuickOverview.vue'
import { toRefs } from 'vue'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
}>()
const { task } = toRefs(props)
</script>
