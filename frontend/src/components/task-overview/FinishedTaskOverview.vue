<template>
  <router-link :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }" class="block">
    <div
      class="leading-none tracking-tight flex items-start justify-between p-2 hover:bg-accent hover:text-accent-foreground group"
      :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
    >
      <div class="flex flex-col justify-center">
        <div class="mb-1 flex items-center gap-1 flex-wrap mr-1">
          <a
            v-if="commitUrl"
            :href="commitUrl"
            target="_blank"
            class="text-muted-foreground hover:underline"
            @click.prevent="openUrl(commitUrl)"
          >
            {{ task.info.revisionId.substring(0, 8) }}:
          </a>
          <span v-else>{{ task.info.revisionId.substring(0, 8) }}: </span>
          <span class="font-medium">{{ task.info.commitMessage }}</span>
          <AutoSelectedGradedMarker :task-id="task.info.taskId" />
          <ManuallyOverrideDialog :task-id="task.info.taskId" />
        </div>
        <TaskQuickOverview class="text-sm" :task="task" />
      </div>
      <div
        class="text-sm text-muted-foreground flex flex-col justify-center items-end self-stretch"
      >
        <span>
          {{ formatTime(task.info.start) }}
        </span>
        <span>
          {{ formatDurationBetween(task.info.start, task.info.end) }}
        </span>
      </div>
    </div>
  </router-link>
</template>

<script setup lang="ts">
import { formatDurationBetween, formatTime, useCommitUrl } from '@/lib/utils.ts'
import AutoSelectedGradedMarker from '@/components/task-overview/AutoSelectedGradedMarker.vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import ManuallyOverrideDialog from '@/components/task-overview/ManuallyOverrideDialog.vue'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import { toRefs } from 'vue'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
}>()
const { task } = toRefs(props)

const { commitUrl } = useCommitUrl(task.value.info.revisionId)

function openUrl(commitUrl: string) {
  window.open(commitUrl, '_blank')
}
</script>
