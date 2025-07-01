<template>
  <router-link :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }" class="block">
    <div
      :class="
        clsx(
          'leading-tight',
          'tracking-tight',
          'flex',
          'flex-wrap',
          'items-start',
          'justify-end',
          'p-2',
          'hover:bg-accent',
          'hover:text-accent-foreground',
          'group',
          'rounded-xl',
          'border',
          'bg-card',
          'text-card-foreground',
        )
      "
    >
      <div class="flex max-w-full flex-col flex-wrap justify-center">
        <div class="mb-1 mr-1 flex max-w-full flex-wrap items-stretch gap-1">
          <a
            v-if="commitUrl"
            :href="commitUrl"
            target="_blank"
            class="text-muted-foreground hover:underline"
            @click.prevent="openUrl(commitUrl)"
          >
            {{ task.info.revisionId.substring(0, 7) }}:
          </a>
          <span v-else class="text-muted-foreground"
            >{{ task.info.revisionId.substring(0, 7) }}:
          </span>
          <span class="max-w-[50ch] overflow-hidden text-ellipsis text-nowrap font-medium">
            {{ task.info.commitMessage }}
          </span>
          <FinalizedGradedMarker :task-id="task.info.taskId" />
          <AutoSelectedGradedMarker v-if="!hideSubmissionButtons" :task-id="task.info.taskId" />
          <ManuallyOverrideDialog v-if="!hideSubmissionButtons" :task-id="task.info.taskId" />
        </div>
        <TaskQuickOverview class="text-sm" :task="task" />
      </div>
      <div class="flex-grow" />
      <div
        class="flex flex-col items-end justify-center self-stretch text-sm text-muted-foreground"
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
import FinalizedGradedMarker from '@/components/task-overview/FinalizedGradedMarker.vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import ManuallyOverrideDialog from '@/components/task-overview/ManuallyOverrideDialog.vue'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import { clsx } from 'clsx'
import { toRefs } from 'vue'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
  hideSubmissionButtons?: boolean
  repoUrl?: string
}>()
const { task, repoUrl } = toRefs(props)

const { commitUrl } = useCommitUrl(task.value.info.revisionId, task.value.info.teamId, repoUrl)

function openUrl(commitUrl: string) {
  window.open(commitUrl, '_blank')
}
</script>
