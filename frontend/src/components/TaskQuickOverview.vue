<template>
  <span v-if="tests && stats" class="space-x-2">
    <span>{{ tests.length }} tests</span>
    <span v-if="stats.finish + stats.finishProv > 0" class="text-green-500">
      {{ stats.finish }}
      <span v-if="stats.finishProv > 0">(+{{ stats.finishProv }})</span> finished
    </span>
    <span v-if="stats.failure + stats.failureProv > 0" class="text-red-500">
      {{ stats.failure }}
      <span v-if="stats.failureProv > 0">(+{{ stats.failureProv }})</span>
      failures
    </span>
    <span v-if="stats.error + stats.errorProv > 0" class="text-red-400">
      {{ stats.error }}
      <span v-if="stats.errorProv > 0">(+{{ stats.errorProv }})</span>
      internal errors
    </span>
    <span v-if="stats.timeout + stats.timeoutProv > 0" class="text-orange-500">
      {{ stats.timeout }}
      <span v-if="stats.timeoutProv > 0">(+{{ stats.timeoutProv }})</span>
      timeouts
    </span>
    <span v-if="stats.abort + stats.abortProv > 0" class="text-gray-500">
      {{ stats.abort }}
      <span v-if="stats.abortProv > 0">(+{{ stats.abortProv }})</span>
      aborted
    </span>
    <span v-if="outdatedTests.length > 0" class="text-muted-foreground">
      but {{ outdatedTests.length }} test{{ outdatedTests.length > 1 ? 's have' : ' has' }} changed
      since
    </span>
  </span>
  <span v-else-if="task.type === 'BuildFailed'" :class="[statusColor(task.status, 'text')]">
    Build did not succeed.
    <span v-if="task.status === 'Aborted'">It was aborted.</span>
    <span v-if="task.status === 'Error'">It ran into an internal error.</span>
  </span>
</template>

<script setup lang="ts">
import { computed, toRefs } from 'vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import { statusColor } from '@/lib/utils.ts'

const props = defineProps<{
  task: FinishedCompilerTaskSummary
}>()
const { task } = toRefs(props)

const tests = computed(() => (task.value.type === 'RanTests' ? task.value.tests : undefined))

const stats = computed(() => {
  if (tests.value === undefined || task.value.type === 'BuildFailed') {
    return undefined
  }

  const abortArr = tests.value.filter((test) => test.output === 'Aborted')
  const errorArr = tests.value.filter((test) => test.output === 'Error')
  const failureArr = tests.value.filter((test) => test.output === 'Failure')
  const finishArr = tests.value.filter((test) => test.output === 'Success')
  const timeoutArr = tests.value.filter((test) => test.output === 'Timeout')

  const abort = abortArr.filter((test) => test.provisionalForCategory === null).length
  const error = errorArr.filter((test) => test.provisionalForCategory === null).length
  const failure = failureArr.filter((test) => test.provisionalForCategory === null).length
  const finish = finishArr.filter((test) => test.provisionalForCategory === null).length
  const timeout = timeoutArr.filter((test) => test.provisionalForCategory === null).length

  const abortProv = abortArr.filter((test) => test.provisionalForCategory !== null).length
  const errorProv = errorArr.filter((test) => test.provisionalForCategory !== null).length
  const failureProv = failureArr.filter((test) => test.provisionalForCategory !== null).length
  const finishProv = finishArr.filter((test) => test.provisionalForCategory !== null).length
  const timeoutProv = timeoutArr.filter((test) => test.provisionalForCategory !== null).length

  return {
    abort,
    abortProv,
    error,
    errorProv,
    failure,
    failureProv,
    finish,
    finishProv,
    timeout,
    timeoutProv,
  }
})
const outdatedTests = computed(() => {
  if (task.value.type !== 'RanTests') {
    return []
  }
  return task.value.outdated
})
</script>
