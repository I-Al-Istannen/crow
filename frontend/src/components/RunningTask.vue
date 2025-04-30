<template>
  <div v-auto-animate class="space-y-2">
    <Card>
      <CardHeader class="pb-2">
        <CardTitle>Task in progress</CardTitle>
        <CardDescription>Watch the live output of a running task</CardDescription>
      </CardHeader>
      <CardContent v-auto-animate>
        <div v-if="status === 'CONNECTING'">Trying to connect to data stream...</div>
        <div v-if="status === 'CLOSED'">Connection lost. Will retry periodically...</div>
        <ol class="list-inside list-decimal">
          <li>
            Your data is being transferred to a runner<span v-if="buildStatus === null">{{
              animatedWaitingDots
            }}</span>
          </li>
          <li v-if="buildStatus !== null">
            Building the compiler<span v-if="buildStatus === 'Started'">{{
              animatedWaitingDots
            }}</span>
          </li>
          <li v-if="buildStatus && buildStatus !== 'Started'">Build completed</li>
          <li v-if="testingStarted">Testing has started</li>
        </ol>
      </CardContent>
    </Card>
    <BuildOutputOverview
      v-if="status === 'OPEN' && buildExecutionOutput"
      :task-or-output="buildExecutionOutput"
    />
    <Card v-if="status === 'OPEN' && testingStarted">
      <CardHeader>
        <CardTitle>Test results</CardTitle>
        <CardDescription>Information about individual tests</CardDescription>
      </CardHeader>
      <CardContent>
        <TestOverviewMatrix of-whom="yours" :tests="tests" />
      </CardContent>
    </Card>
  </div>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  type ExecutingTest,
  type ExecutionOutput,
  type FinishedExecution,
  type FinishedTest,
  RunnerUpdateMessageSchema,
  type TaskId,
} from '@/types.ts'
import { computed, ref } from 'vue'
import { useIntervalFn, useWebSocket } from '@vueuse/core'
import { BACKEND_URL } from '@/data/fetching.ts'
import BuildOutputOverview from '@/components/BuildOutputOverview.vue'
import TestOverviewMatrix from '@/components/TestOverviewMatrix.vue'
import { storeToRefs } from 'pinia'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const buildStatus = ref<'Started' | FinishedExecution | null>(null)
const testingStarted = ref(false)
const tests = ref<(FinishedTest | ExecutingTest)[]>([])
const animatedWaitingDotsCounter = ref(-3)
const animatedWaitingDots = computed(() =>
  '.'.repeat(3 - Math.abs(animatedWaitingDotsCounter.value)),
)

useIntervalFn(
  () => {
    if (animatedWaitingDotsCounter.value >= 3) {
      animatedWaitingDotsCounter.value = -3
    } else {
      animatedWaitingDotsCounter.value++
    }
  },
  500,
  { immediate: true },
)

const props = defineProps<{
  taskId: TaskId
}>()

const { token } = storeToRefs(useUserStore())

const buildExecutionOutput = computed<ExecutionOutput | undefined>(() => {
  if (buildStatus.value && buildStatus.value !== 'Started') {
    return { type: 'Success', ...buildStatus.value }
  }
  return undefined
})

const emit = defineEmits<{
  connectionLost: []
}>()

const websocketUrl = computed(
  () => `${BACKEND_URL}/tasks/${encodeURIComponent(props.taskId)}/stream`,
)
const { status } = useWebSocket(websocketUrl, {
  autoReconnect: true,
  immediate: true,
  onDisconnected: async () => {
    emit('connectionLost')
  },
  onConnected: (ws) => {
    // login
    ws.send(token.value!)
  },
  onMessage: (ws, wsEvent) => {
    const data = JSON.parse(wsEvent.data)
    if ('error' in data) {
      toast.error(data.error)
      return
    }
    const event = RunnerUpdateMessageSchema.parse(data)
    const update = event.update
    switch (update.type) {
      case 'Done': {
        toast.success('Task completed')
        ws.close()
        break
      }
      case 'StartedBuild': {
        buildStatus.value = 'Started'
        break
      }
      case 'FinishedBuild': {
        buildStatus.value = update.result
        break
      }
      case 'AllTests': {
        for (const newTestId of update.tests) {
          tests.value.push({
            testId: newTestId,
            status: 'Queued',
          })
        }
        break
      }
      case 'StartedTest': {
        testingStarted.value = true

        const existing = tests.value.findIndex((test) => test.testId === update.testId)
        if (existing !== -1) {
          tests.value[existing] = {
            status: 'Started',
            testId: update.testId,
          }
        } else {
          tests.value.push({
            status: 'Started',
            testId: update.testId,
          })
        }
        break
      }
      case 'FinishedTest': {
        const existing = tests.value.findIndex((test) => test.testId === update.result.testId)
        if (existing !== -1) {
          tests.value[existing] = update.result
        } else {
          tests.value.push(update.result)
        }
        break
      }
    }
  },
})
</script>
