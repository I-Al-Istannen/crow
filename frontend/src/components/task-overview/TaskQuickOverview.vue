<template>
  <span v-if="stats" class="space-x-2">
    <span>{{ stats.total.total }} tests</span>
    <span v-if="stats.success.total > 0" class="text-green-500">
      {{ stats.success.normal }}
      <span v-if="stats.success.provisional > 0">(+{{ stats.success.provisional }})</span> finished
    </span>
    <span v-if="stats.failure.total > 0" class="text-red-500">
      {{ stats.failure.normal }}
      <span v-if="stats.failure.provisional > 0">(+{{ stats.failure.provisional }})</span>
      failures
    </span>
    <span v-if="stats.error.total > 0" class="text-red-400">
      {{ stats.error.normal }}
      <span v-if="stats.error.provisional > 0">(+{{ stats.error.provisional }})</span>
      internal errors
    </span>
    <span v-if="stats.timeout.total > 0" class="text-orange-500">
      {{ stats.timeout.normal }}
      <span v-if="stats.timeout.provisional > 0">(+{{ stats.timeout.provisional }})</span>
      timeouts
    </span>
    <span v-if="stats.abort.total > 0" class="text-gray-500">
      {{ stats.abort.normal }}
      <span v-if="stats.abort.provisional > 0">(+{{ stats.abort.provisional }})</span>
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

const stats = computed(() => (task.value.type === 'RanTests' ? task.value.statistics : undefined))

const outdatedTests = computed(() => {
  if (task.value.type !== 'RanTests') {
    return []
  }
  return task.value.outdated
})
</script>
