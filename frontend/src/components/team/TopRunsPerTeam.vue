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
        <Table class="w-fit" v-if="topTasks.size > 3">
          <TableHeader>
            <TableHead class="text-center">Place</TableHead>
            <TableHead class="text-center">Team</TableHead>
            <TableHead class="text-center">Tests</TableHead>
            <TableHead class="text-center">Time</TableHead>
            <TableHead class="text-center">Task</TableHead>
          </TableHeader>
          <TableBody>
            <TableRow v-for="([teamId, task], index) in sortedTeams" :key="teamId">
              <TableCell class="py-0 text-center">{{ index + 1 }}</TableCell>
              <TableCell class="py-0">{{ task.teamName }}</TableCell>
              <TableCell class="py-0">
                <TaskQuickOverview class="text-sm" :task="task" />
              </TableCell>
              <TableCell class="py-0">{{ task.info.start.toLocaleString() }}</TableCell>
              <TableCell class="py-0">
                <RouterLink :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }">
                  <Button variant="link">To the task</Button>
                </RouterLink>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import { computed } from 'vue'
import { queryTopTaskPerTeam } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const { data: topTasks, isLoading, failureCount, failureReason } = queryTopTaskPerTeam()

const sortedTeams = computed(() => {
  if (!topTasks.value) {
    return undefined
  }
  const result = [...topTasks.value.entries()]
  result.sort((a, b) => {
    const cmp = finishedTests(b[1]) - finishedTests(a[1])
    if (cmp !== 0) {
      return cmp
    }
    return a[0].localeCompare(b[0])
  })

  return result
})

function finishedTests(task: FinishedCompilerTaskSummary) {
  const tests = task.type === 'RanTests' ? task.tests : undefined
  return tests?.filter((test) => test.output === 'Success')?.length || 0
}
</script>
