<template>
  <div v-if="buildOutput">
    {{ subject }} finished after
    <span class="font-mono font-bold"> {{ formatDuration(buildOutput.runtime) }} </span>.
    <span v-if="buildOutput.exitStatus != null">
      {{ subject }} exited with status
      <span class="font-mono font-bold">{{ buildOutput.exitStatus }} </span>.
      <span v-if="buildOutput.exitStatus === 0">This is a good sign.</span>
    </span>
    <span v-if="buildOutput.error !== undefined">
      Unfortunately, crow encountered an internal error.
    </span>
    <Accordion type="multiple">
      <AccordionItem value="error" v-if="buildOutput.error !== undefined">
        <AccordionTrigger>Internal error</AccordionTrigger>
        <AccordionContent>
          <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto">{{
            buildOutput.error
          }}</pre>
        </AccordionContent>
      </AccordionItem>
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
          <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto"><span
            class="block"
            v-for="(line, index) in stderrLines"
            :class="{
              'text-red-500': line.startsWith('-'),
              'text-green-600': line.startsWith('+'),
              'text-violet-500': line.startsWith('@@'),
            }"
            :key="index">{{ line }}</span></pre>
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

const stderrLines = computed(() => {
  return buildOutput.value.stderr.split('\n')
})

function getBuildOutput(task: ExecutionOutput): {
  stdout: string
  stderr: string
  runtime: number
  exitStatus?: number | null
  error?: string
} {
  if (task.type === 'Error') {
    return {
      stdout: '',
      stderr: '',
      runtime: task.runtime,
      error: task.message,
    }
  }
  return task
}
</script>
