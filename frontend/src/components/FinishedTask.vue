<template>
  <Card>
    <CardHeader class="pb-2">
      <CardTitle>
        <span v-if="taskSummary">{{ taskSummary.info.commitMessage }}</span>
        <span v-else>Task summary</span>
      </CardTitle>
      <CardDescription>
        <span v-if="taskSummary" class="break-all">{{ taskSummary.info.revisionId }}</span>
        <span v-else>A quick summary of the most important points</span>
      </CardDescription>
    </CardHeader>
    <CardContent v-if="isLoading">Loading task data...</CardContent>
    <CardContent v-if="isFetched && task === null">
      Task not found yet. Try waiting a few seconds.
    </CardContent>
    <CardContent v-if="isFetched && taskSummary">
      <TaskQuickOverview :task="taskSummary" />
    </CardContent>
  </Card>

  <BuildOutputOverview :task-or-output="task" v-if="task" />
  <TestOverview :tests="tests" of-whom="yours" v-if="tests && task" />
</template>
<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  type FinishedCompilerTask,
  type FinishedCompilerTaskSummary,
  type FinishedTestSummary,
  type TaskId,
  toExecutionStatus,
} from '@/types.ts'
import BuildOutputOverview from '@/components/BuildOutputOverview.vue'
import TaskQuickOverview from '@/components/TaskQuickOverview.vue'
import TestOverview from '@/components/TestOverview.vue'
import { computed } from 'vue'
import { queryTask } from '@/data/network.ts'

const props = defineProps<{
  taskId: TaskId
}>()
const { taskId } = props

const { data: task, isFetched, isLoading } = queryTask(taskId)
const taskSummary = computed(() => (task.value ? toSummary(task.value) : undefined))

const tests = computed(() => {
  if (!task.value || task.value.type !== 'RanTests') {
    return undefined
  }
  return task.value.tests
})

function toSummary(task: FinishedCompilerTask): FinishedCompilerTaskSummary {
  if (task.type == 'BuildFailed') {
    return {
      type: 'BuildFailed',
      info: task.info,
      status: task.buildOutput.type,
    }
  }
  const tests: FinishedTestSummary[] = task.tests.map((test) => ({
    output: toExecutionStatus(test.output),
    testId: test.testId,
  }))

  return {
    type: 'RanTests',
    tests,
    info: task.info,
    outdated: task.outdated,
  }
}
</script>
