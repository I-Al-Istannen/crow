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
      <div class="flex flex-col justify-center max-w-full flex-wrap">
        <div class="mb-1 flex items-stretch gap-1 flex-wrap mr-1 max-w-full">
          <a
            v-if="commitUrl && task.info.teamId == team?.id"
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
          <span class="font-medium text-ellipsis overflow-hidden text-nowrap max-w-[50ch]">
            {{ task.info.commitMessage }}
          </span>
          <AutoSelectedGradedMarker v-if="!hideSubmissionButtons" :task-id="task.info.taskId" />
          <ManuallyOverrideDialog v-if="!hideSubmissionButtons" :task-id="task.info.taskId" />
        </div>
        <TaskQuickOverview class="text-sm" :task="task" />
      </div>
      <div class="flex-grow" />
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
import { clsx } from 'clsx'
import { storeToRefs } from 'pinia'
import { toRefs } from 'vue'
import { useUserStore } from '@/stores/user.ts'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
  hideSubmissionButtons?: boolean
}>()
const { task } = toRefs(props)
const { team } = storeToRefs(useUserStore())

const { commitUrl } = useCommitUrl(task.value.info.revisionId)

function openUrl(commitUrl: string) {
  window.open(commitUrl, '_blank')
}
</script>
