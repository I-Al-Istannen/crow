<template>
  <PageContainer>
    <Card>
      <CardHeader class="pb-2">
        <CardTitle>Task summary</CardTitle>
        <CardDescription> A quick summary of the most important points</CardDescription>
      </CardHeader>
      <CardContent v-if="isLoading"> Loading task data...</CardContent>
      <CardContent v-if="isFetched && task === null">
        Task not found. Try refreshing in a minute or so?
      </CardContent>
      <CardContent v-if="isFetched && taskSummary">
        <TaskQuickOverview :task="taskSummary" />
      </CardContent>
    </Card>

    <BuildOutputOverview :task="task" v-if="task" />
    <TestOverview :tests="tests" v-if="tests && task" />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  type FinishedCompilerTask,
  type FinishedCompilerTaskSummary,
  type FinishedTestSummary,
  type TaskId,
} from '@/types.ts'
import BuildOutputOverview from '@/components/BuildOutputOverview.vue'
import PageContainer from '@/components/PageContainer.vue'
import TaskQuickOverview from '@/components/TaskQuickOverview.vue'
import TestOverview from '@/components/TestOverview.vue'
import { computed } from 'vue'
import { queryTask } from '@/data/network.ts'
import { useRoute } from 'vue-router'

const route = useRoute()
const taskId = computed(() => (route.params?.taskId ? (route.params.taskId as TaskId) : undefined))

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
    output: test.output.type,
    testId: test.testId,
  }))

  return {
    type: 'RanTests',
    tests,
    info: task.info,
  }
}
</script>
