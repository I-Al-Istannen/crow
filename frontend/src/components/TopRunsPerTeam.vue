<template>
  <Card>
    <CardHeader>
      <CardTitle>Top runs per Team</CardTitle>
      <CardDescription>The best runs of each team. Being at the top is good :)</CardDescription>
    </CardHeader>
    <CardContent v-auto-animate>
      <div v-if="isLoading">Loading tasks</div>
      <div v-if="topTasks" class="space-y-2 -mt-2">
        <div v-for="[team, task] in sortedTeams" :key="team" class="flex flex-col">
          <span class="font-medium mb-1">{{ team }}:</span>
          <FinishedTaskOverview :task="task" class="ml-2" />
        </div>
      </div>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import FinishedTaskOverview from '@/components/FinishedTaskOverview.vue'
import { computed } from 'vue'
import { queryTopTaskPerTeam } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const { data: topTasks, isLoading } = queryTopTaskPerTeam()
const sortedTeams = computed(() => {
  if (!topTasks.value) {
    return undefined
  }
  const result = [...topTasks.value.entries()]
  result.sort((a, b) => finishedTests(b[1]) - finishedTests(a[1]))

  return result
})

function finishedTests(task: FinishedCompilerTaskSummary) {
  const tests = task.type === 'RanTests' ? task.tests : undefined
  return tests?.filter((test) => test.output === 'Finished')?.length || 0
}
</script>
