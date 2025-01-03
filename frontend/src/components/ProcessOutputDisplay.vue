<template>
  <div v-if="buildOutput">
    {{ subject }} finished after
    <span class="font-mono font-bold"> {{ formatDuration(buildOutput.runtime) }} </span>.
    <span v-if="buildOutput.exitStatus != null">
      {{ subject }} exited with status
      <span class="font-mono font-bold">{{ buildOutput.exitStatus }} </span>.
      <span v-if="buildOutput.exitStatus === 0">This is a good sign.</span>
    </span>
    <Accordion type="multiple">
      <AccordionItem value="stdout" v-if="buildOutput.stdout.length > 0">
        <AccordionTrigger>Stdout</AccordionTrigger>
        <AccordionContent>
          <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto">{{
            buildOutput.stdout
          }}</pre>
        </AccordionContent>
      </AccordionItem>
      <AccordionItem value="stderr" v-if="buildOutput.stderr.length > 0">
        <AccordionTrigger>Stderr</AccordionTrigger>
        <AccordionContent>
          <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto">{{
            buildOutput.stderr
          }}</pre>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  </div>
</template>

<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import { computed, toRefs } from 'vue'
import type { ExecutionOutput } from '@/types.ts'
import { formatDuration } from '@/lib/utils.ts'

const props = defineProps<{
  output: ExecutionOutput
  subject: string
}>()

const { output, subject } = toRefs(props)

const buildOutput = computed(() => getBuildOutput(output.value))

function getBuildOutput(task: ExecutionOutput):
  | {
      stdout: string
      stderr: string
      runtime: number
      exitStatus?: number | null
    }
  | undefined {
  if (task.type === 'Error') {
    return undefined
  }
  return task
}
</script>
