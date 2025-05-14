<template>
  <Card v-if="isLoading || (sortedTeams?.length || 0) > 0">
    <CardHeader>
      <CardTitle>Top runs per Team</CardTitle>
      <CardDescription>The best runs of each team. Being at the top is good :)</CardDescription>
    </CardHeader>
    <CardContent v-auto-animate>
      <DataLoadingExplanation
        :is-loading="isLoading"
        :failure-count="failureCount"
        :failure-reason="failureReason"
      />
      <div v-if="topTasks" class="space-y-2 -mt-2">
        <TopTaskTable v-if="topTasks.size >= 2" :sorted-teams="sortedTeams || []" />
        <template v-else>
          <div v-for="[team, task] in sortedTeams" :key="team" class="flex flex-col">
            <span class="font-medium mb-1">{{ task.teamName }}:</span>
            <FinishedTaskOverview :task="task" class="ml-2" />
          </div>
        </template>
      </div>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import {
  type ApiFinishedCompilerTaskSummary,
  type FinishedCompilerTaskSummary,
  type TeamId,
} from '@/types.ts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import TopTaskTable from '@/components/top-tasks/TopTaskTable.vue'
import { computed } from 'vue'
import { queryTopTaskPerTeam } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

type TaskWithFinishedTests = [number, [TeamId, ApiFinishedCompilerTaskSummary]][]

const { data: topTasks, isLoading, failureCount, failureReason } = queryTopTaskPerTeam()

const sortedTeams = computed(() => {
  if (!topTasks.value) {
    return undefined
  }
  const result: TaskWithFinishedTests = Array.from(topTasks.value.entries()).map(([team, task]) => [
    finishedTests(task),
    [team, task],
  ])

  result.sort((a, b) => {
    const cmp = b[0] - a[0]
    if (cmp !== 0) {
      return cmp
    }
    return a[1][0].localeCompare(b[1][0])
  })

  return result.map(([_, it]) => it)
})

function finishedTests(task: FinishedCompilerTaskSummary): number {
  const tests = task.type === 'RanTests' ? task.tests : undefined
  return tests?.filter((test) => test.output === 'Success')?.length || 0
}
</script>
