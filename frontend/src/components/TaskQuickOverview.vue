<template>
  <span v-if="tests && testStatistics" class="space-x-2">
    <span>{{ tests.length }} tests</span>
    <span v-if="testStatistics.finish > 0" class="text-green-500">
      {{ testStatistics.finish }} finished
    </span>
    <span v-if="testStatistics.error > 0" class="text-red-500">
      {{ testStatistics.error }} errors
    </span>
    <span v-if="testStatistics.timeout > 0" class="text-orange-500">
      {{ testStatistics.timeout }} timeouts
    </span>
    <span v-if="testStatistics.abort > 0" class="text-gray-500">
      {{ testStatistics.abort }} aborted
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

const testStatistics = computed(() => {
  if (tests.value === undefined) {
    return undefined
  }

  const finish = tests.value.filter((test) => test.output === 'Finished').length
  const abort = tests.value.filter((test) => test.output === 'Aborted').length
  const error = tests.value.filter((test) => test.output === 'Error').length
  const timeout = tests.value.filter((test) => test.output === 'Timeout').length

  return { finish, abort, error, timeout }
})
</script>
