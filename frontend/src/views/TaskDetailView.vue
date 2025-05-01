<template>
  <PageContainer>
    <FinishedTask
      v-if="taskStatus === 'finished' && taskId"
      :task-id="taskId"
      :initial-view="wasOnceRunning ? 'matrix' : undefined"
    />
    <RunningTask
      v-else-if="taskStatus === 'running' && taskId"
      :task-id="taskId"
      @connection-lost="resume"
    />
    <Card v-else>
      <CardHeader class="pb-2">
        <CardTitle>
          <div v-if="queuedTask" class="flex items-center">
            <span>{{ queuedTask.commitMessage }}</span>
            <TaskExternalLinkIcon class="ml-2" :revision="queuedTask.revision" />
          </div>
          <span v-else>Task detail</span>
        </CardTitle>
        <CardDescription>
          <span v-if="queuedTask">{{ queuedTask.revision }}</span>
          <span v-else>View information about running or finished tasks</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div v-if="hasLoadedOnce && taskStatus === null" class="mb-2">
          It looks like no task with this ID exists. Maybe it isn't running or finished yet?
        </div>
        <!-- Below to not cause as many layout shifts-->
        <DataLoadingExplanation
          :is-loading="isFetching"
          :failure-count="failureCount"
          :failure-reason="failureReason"
        />
        <div v-if="taskStatus === 'queued' && queuedTask">
          This task has been queued since
          <span class="font-medium">
            {{ formatTime(queuedTask.insertTime) }}
          </span>
          <span class="text-muted-foreground">
            ({{ formatDurationBetween(queuedTask.insertTime, new Date(currentTimeMs)) }}).
          </span>
        </div>
        <div v-if="taskStatus === 'queued' && queuedTask && lastUpdate">
          The last update of this page was at
          <span class="font-medium"> {{ formatTime(lastUpdate) }} </span>.
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { type TaskId, type WorkItem } from '@/types.ts'
import { computed, onMounted, ref, watch } from 'vue'
import { fetchQueuedTask, fetchRunningTaskExists, fetchTaskExists } from '@/data/network.ts'
import { formatDurationBetween, formatTime } from '../lib/utils.ts'
import { useIntervalFn, useTimestamp } from '@vueuse/core'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTask from '@/components/task-detail/FinishedTask.vue'
import PageContainer from '@/components/PageContainer.vue'
import RunningTask from '@/components/task-detail/RunningTask.vue'
import TaskExternalLinkIcon from '@/components/task-detail/TaskExternalLinkIcon.vue'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const route = useRoute()
const taskId = computed(() => (route.params?.taskId ? (route.params.taskId as TaskId) : undefined))
const { loggedIn } = storeToRefs(useUserStore())

const taskStatus = ref<'queued' | 'running' | 'finished' | null>(null)
const isFetching = ref(false)
const hasLoadedOnce = ref(false)
const initDone = ref(false)
const queuedTask = ref<WorkItem | null>(null)
const lastUpdate = ref<Date | null>(null)
const failureReason = ref<Error | null>(null)
const failureCount = ref<number>(0)
const wasOnceRunning = ref(false)

const { pause, resume } = useIntervalFn(
  async () => {
    await iteration()
  },
  2000,
  { immediateCallback: true },
)
const currentTimeMs = useTimestamp({ interval: 500 })

watch([taskId, loggedIn], () => {
  resume()
})

watch(
  taskStatus,
  (status) => {
    if (status === 'running') {
      wasOnceRunning.value = true
    }
  },
  { immediate: true },
)

onMounted(async () => {
  initDone.value = true
  await iteration()
})

async function iteration() {
  // The call from the initial trigger can not use `pause`, so work around it
  if (!initDone.value) {
    return
  }
  try {
    // If we are not logged in, do not have a task, or it is finished, we stop polling
    // Nothing can change by itself.
    if (!loggedIn.value || !taskId.value || taskStatus.value === 'finished') {
      pause()
      return
    }
    // If we are running, we get our updates from the websocket listener, we can stop polling
    if (taskStatus.value === 'running') {
      pause()
    }

    isFetching.value = true
    for (const func of getUpdateOrder()) {
      if (await func(taskId.value)) {
        break
      }
    }
    lastUpdate.value = new Date()
    failureReason.value = null
    failureCount.value = 0
    isFetching.value = false
    hasLoadedOnce.value = true
  } catch (e) {
    failureReason.value = e as Error
    failureCount.value++
  }
}

function getUpdateOrder() {
  if (taskStatus.value === 'queued') {
    return [updateFromQueued, updateFromRunning, updateFromFinished]
  }
  if (taskStatus.value === 'running') {
    return [updateFromRunning, updateFromFinished, updateFromQueued]
  }

  return [updateFromFinished, updateFromQueued, updateFromRunning]
}

async function updateFromQueued(taskId: TaskId) {
  const result = await fetchQueuedTask(taskId)
  if (result !== null) {
    taskStatus.value = 'queued'
    queuedTask.value = result
    return true
  }
  return false
}

async function updateFromRunning(taskId: TaskId) {
  if (await fetchRunningTaskExists(taskId)) {
    taskStatus.value = 'running'
    return true
  }
  return false
}

async function updateFromFinished(taskId: TaskId) {
  if (await fetchTaskExists(taskId)) {
    taskStatus.value = 'finished'
    return true
  }
  return false
}
</script>
