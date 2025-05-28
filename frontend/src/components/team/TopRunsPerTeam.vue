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
import { type ApiFinishedCompilerTaskSummary, type TeamId } from '@/types.ts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import TopTaskTable from '@/components/top-tasks/TopTaskTable.vue'
import { computed } from 'vue'
import { queryTopTaskPerTeam } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

type TaskWithFinishedTest = {
  passingTests: number
  task: ApiFinishedCompilerTaskSummary
  team: TeamId
}

const { data: topTasks, isLoading, failureCount, failureReason } = queryTopTaskPerTeam()

const sortedTeams = computed<[TeamId, ApiFinishedCompilerTaskSummary][] | undefined>(() => {
  if (!topTasks.value) {
    return undefined
  }
  const result: TaskWithFinishedTest[] = Array.from(topTasks.value.entries()).map(
    ([team, task]) => ({
      passingTests: task.type === 'RanTests' ? task.statistics.success.total : 0,
      task,
      team,
    }),
  )

  result.sort((a, b) => {
    let cmp = b.passingTests - a.passingTests
    if (cmp !== 0) {
      return cmp
    }
    cmp = a.task.info.start.getTime() - b.task.info.start.getTime()
    if (cmp !== 0) {
      return cmp
    }
    return a.team.localeCompare(b.team)
  })

  return result.map((it) => [it.team, it.task])
})
</script>
