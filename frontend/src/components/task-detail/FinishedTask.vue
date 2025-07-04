<template>
  <Card>
    <CardHeader class="pb-2">
      <CardTitle class="flex justify-between">
        <div v-if="taskSummary" class="flex items-center">
          <span>{{ taskSummary.info.commitMessage.split('\n')[0] }}</span>
          <TaskExternalLinkIcon
            class="ml-2"
            :revision="taskSummary.info.revisionId"
            :teamId="taskSummary.info.teamId"
          />
        </div>
        <span v-else>Task summary</span>
        <div v-if="taskSummary" class="ml-5 text-muted-foreground">
          <RouterLink
            v-if="isAdmin"
            :to="{ name: 'team-info', params: { teamId: taskSummary.info.teamId } }"
            class="cursor-pointer hover:underline"
          >
            by the {{ taskSummary.info.teamId }}
          </RouterLink>
          <span v-else>{{ taskSummary.info.teamId }}</span>
        </div>
      </CardTitle>
      <CardDescription>
        <span v-if="taskSummary" class="break-all">{{ taskSummary.info.revisionId }}</span>
        <span v-else>A quick summary of the most important points</span>
      </CardDescription>
    </CardHeader>
    <CardContent v-if="isLoading">
      <DataLoadingExplanation
        :isLoading="isLoading"
        :failureReason="failureReason"
        :failureCount="failureCount"
      />
    </CardContent>
    <CardContent v-if="isFetched && task === null">
      Task not found yet. Try waiting a few seconds.
    </CardContent>
    <CardContent v-if="isFetched && taskSummary">
      <TaskQuickOverview :task="taskSummary" />
    </CardContent>
  </Card>

  <FinishedTestDetailDialog
    :test="clickedTest"
    of-whom="yours"
    v-model:dialog-open="dialogOpen"
    :outdated="outdatedTests.includes(clickedTest?.testId || '')"
  />
  <BuildOutputOverview :task-or-output="task" v-if="task" />

  <Card v-if="tests && task">
    <CardHeader class="flex flex-col items-start justify-between sm:flex-row sm:items-center">
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
        :tests="Array.from(tests.values())"
        @test-clicked="handleTestClicked($event.testId)"
      />
      <TestOverviewMatrix
        v-else
        :tests="sortedTestSummaries!"
        @test-clicked="handleTestClicked($event.testId)"
        is-finished
      />
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
  type TestId,
  toFinishedTestSummary,
} from '@/types.ts'
import { computed, ref, watch } from 'vue'
import BuildOutputOverview from '@/components/task-detail/BuildOutputOverview.vue'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTestDetailDialog from '@/components/test-view/FinishedTestDetailDialog.vue'
import TaskExternalLinkIcon from '@/components/task-detail/TaskExternalLinkIcon.vue'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import TestOverviewMatrix from '@/components/task-detail/TestOverviewMatrix.vue'
import TestOverviewTable from '@/components/task-detail/TestOverviewTable.vue'
import { queryTask } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useTitle } from '@vueuse/core'
import { useUserStore } from '@/stores/user.ts'

const props = defineProps<{
  taskId: TaskId
  initialView?: 'matrix' | 'table'
}>()
const { taskId } = props

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)
const showTableView = ref<boolean>(props.initialView !== 'matrix')

const { isAdmin } = storeToRefs(useUserStore())
const { data: task, isFetched, isLoading, failureReason, failureCount } = queryTask(taskId)
const taskSummary = computed(() => (task.value ? toSummary(task.value) : undefined))

const title = useTitle(undefined, { restoreOnUnmount: false, titleTemplate: '%s - crow' })
watch(task, (newTask) => {
  if (newTask) {
    title.value = 'Task ' + newTask.info.revisionId.substring(0, 7)
  }
})

const tests = computed<Map<TestId, FinishedTest> | undefined>(() => {
  if (!task.value || task.value.type !== 'RanTests') {
    return undefined
  }
  const tests = new Map()
  for (const test of task.value.tests) {
    tests.set(test.testId, test)
  }
  return tests
})

const sortedTestSummaries = computed<FinishedTestSummary[] | undefined>(() => {
  if (!tests.value) {
    return undefined
  }
  return Array.from(tests.value.values())
    .sort((a, b) => a.testId.localeCompare(b.testId))
    .map(toFinishedTestSummary)
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

  return {
    type: 'RanTests',
    info: task.info,
    outdated: task.outdated,
    statistics: task.statistics,
  }
}

function handleTestClicked(id: TestId) {
  const test = tests.value?.get(id)
  if (!test) {
    return
  }
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
