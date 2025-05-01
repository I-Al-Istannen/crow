<template>
  <Card>
    <CardHeader class="pb-2">
      <CardTitle>
        <div v-if="taskSummary" class="flex items-center">
          <span>{{ taskSummary.info.commitMessage }}</span>
          <TaskExternalLinkIcon class="ml-2" :revision="taskSummary.info.revisionId" />
        </div>
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

  <FinishedTestDetailDialog :test="clickedTest" of-whom="yours" v-model:dialog-open="dialogOpen" />
  <BuildOutputOverview :task-or-output="task" v-if="task" />

  <Card v-if="tests && task">
    <CardHeader class="flex flex-col sm:flex-row justify-between items-start sm:items-center">
      <div class="flex flex-col gap-y-1.5">
        <CardTitle>Test results</CardTitle>
        <CardDescription>Information about individual tests</CardDescription>
      </div>
      <div>
        <Button variant="link" @click="showTableView = !showTableView">
          <span v-if="showTableView">Switch to Matrix view</span>
          <span v-else>Switch to Table view</span>
        </Button>
      </div>
    </CardHeader>
    <CardContent>
      <TestOverviewTable
        v-if="showTableView"
        :outdated="outdatedTests"
        :tests="tests"
        @test-clicked="handleTestClicked"
      />
      <TestOverviewMatrix v-else :tests="tests" @test-clicked="handleTestClicked" />
    </CardContent>
  </Card>
</template>
<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  type FinishedCompilerTask,
  type FinishedCompilerTaskSummary,
  type FinishedTest,
  type FinishedTestSummary,
  type TaskId,
  toExecutionStatus,
} from '@/types.ts'
import { computed, ref } from 'vue'
import BuildOutputOverview from '@/components/task-detail/BuildOutputOverview.vue'
import { Button } from '@/components/ui/button'
import FinishedTestDetailDialog from '@/components/test-view/FinishedTestDetailDialog.vue'
import TaskExternalLinkIcon from '@/components/task-detail/TaskExternalLinkIcon.vue'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import TestOverviewMatrix from '@/components/task-detail/TestOverviewMatrix.vue'
import TestOverviewTable from '@/components/task-detail/TestOverviewTable.vue'
import { queryTask } from '@/data/network.ts'

const props = defineProps<{
  taskId: TaskId
  initialView?: 'matrix' | 'table'
}>()
const { taskId } = props

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)
const showTableView = ref<boolean>(props.initialView !== 'matrix')

const { data: task, isFetched, isLoading } = queryTask(taskId)
const taskSummary = computed(() => (task.value ? toSummary(task.value) : undefined))

const tests = computed(() => {
  if (!task.value || task.value.type !== 'RanTests') {
    return undefined
  }
  return task.value.tests
})

const outdatedTests = computed(() => {
  if (task.value?.type !== 'RanTests') {
    return []
  }
  return task.value.outdated
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
    provisionalForCategory: test.provisionalForCategory,
  }))
  return {
    type: 'RanTests',
    tests,
    info: task.info,
    outdated: task.outdated,
  }
}

function handleTestClicked(test: FinishedTest) {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
