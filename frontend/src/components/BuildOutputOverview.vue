<template>
  <Card>
    <CardHeader>
      <CardTitle>Build results</CardTitle>
      <CardDescription>Output from building your compiler</CardDescription>
    </CardHeader>
    <CardContent v-if="buildOutput">
      <ProcessOutputDisplay subject="Your compiler build" :output="buildOutput" />
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { ExecutionOutput, FinishedCompilerTask } from '@/types.ts'
import { computed, toRefs } from 'vue'
import ProcessOutputDisplay from '@/components/ProcessOutputDisplay.vue'

const props = defineProps<{
  task: FinishedCompilerTask
}>()

const { task } = toRefs(props)

const buildOutput = computed(() => getBuildOutput(task.value))

function getBuildOutput(task: FinishedCompilerTask): ExecutionOutput {
  if (task.type === 'BuildFailed') {
    return task.buildOutput
  }
  return {
    type: 'Finished',
    ...task.buildOutput,
  }
}
</script>
