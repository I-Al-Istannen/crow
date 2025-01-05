<template>
  <PageContainer v-auto-animate>
    <FinishedTask v-if="taskStatus === 'finished' && taskId" :task-id="taskId" />
    <RunningTaskOverview
      v-else-if="taskStatus === 'running' && taskId"
      :task-id="taskId"
      @connection-lost="resume"
    />
    <Card v-else>
      <CardHeader class="pb-2">
        <CardTitle>Task detail</CardTitle>
        <CardDescription>View information about running or finished tasks</CardDescription>
      </CardHeader>
      <CardContent v-auto-animate>
        <div v-if="hasTriedFetching && taskStatus === null">
          It looks like no task with this ID exists. Maybe it isn't running or finished yet?
        </div>
        <div v-if="taskStatus === 'queued' && queuedTask">
          This task is queued since
          <span class="font-medium">
            {{ formatTime(queuedTask.insertTime) }}
          </span>
          <span class="text-muted-foreground">
            ({{ formatDurationBetween(queuedTask.insertTime, new Date(currentTimeMs)) }}).
          </span>
        </div>
        <div v-if="taskStatus === 'queued' && queuedTask && lastUpdate">
          The last update of this page was at
          <span class="font-medium">
            {{ formatTime(lastUpdate) }}
          </span>.
        </div>
        <div v-if="isFetching && !hasTriedFetching">Loading data...</div>
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
import FinishedTask from '@/components/FinishedTask.vue'
import PageContainer from '@/components/PageContainer.vue'
import RunningTaskOverview from '@/components/RunningTask.vue'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const route = useRoute()
const taskId = computed(() => (route.params?.taskId ? (route.params.taskId as TaskId) : undefined))
const { loggedIn } = storeToRefs(useUserStore())

const taskStatus = ref<'queued' | 'running' | 'finished' | null>(null)
const hasTriedFetching = ref(false)
const isFetching = ref(false)
const initDone = ref(false)
const queuedTask = ref<WorkItem | null>(null)
const lastUpdate = ref<Date | null>(null)

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
  } finally {
    hasTriedFetching.value = true
    isFetching.value = false
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
