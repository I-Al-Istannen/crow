<template>
  <Card v-if="sortedTasks">
    <CardHeader>
      <CardTitle>All tasks</CardTitle>
      <CardDescription>All tasks for a team, sorted by time</CardDescription>
    </CardHeader>
    <CardContent class="space-y-1">
      <DataLoadingExplanation
        :isLoading="tasksIsLoading"
        :failureCount="tasksFailureCount"
        :failureReason="tasksFailureReason"
      />
      <FinishedTaskOverview
        v-for="task in sortedTasks"
        :task="task"
        :key="task.info.taskId"
        :repoUrl="repoUrl"
        hide-submission-buttons
        class="ml-2"
      />
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { computed, toRefs } from 'vue'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import type { TeamId } from '@/types.ts'
import { queryTasksOfTeam } from '@/data/network.ts'

const props = defineProps<{
  teamId: TeamId
  repoUrl?: string
}>()
const { teamId } = toRefs(props)

const {
  data: tasks,
  failureCount: tasksFailureCount,
  failureReason: tasksFailureReason,
  isLoading: tasksIsLoading,
} = queryTasksOfTeam(teamId)

const sortedTasks = computed(() => {
  if (!tasks.value) {
    return undefined
  }
  // compare the slice by info.start
  return tasks.value.slice().sort((a, b) => b.info.start.getTime() - a.info.start.getTime())
})
</script>
