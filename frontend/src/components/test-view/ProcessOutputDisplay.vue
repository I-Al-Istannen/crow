<template>
  <div v-if="buildOutput">
    <span class="font-semibold">{{ subject }}</span> finished after
    <span class="font-mono font-bold"> {{ formatDuration(buildOutput.runtime) }} </span>.
    <span v-if="buildOutput.exitStatus != null">
      {{ subject }} exited with status
      <span class="font-mono font-bold">{{ buildOutput.exitStatus }}</span
      >.
    </span>
    <span v-if="buildOutput.error !== undefined">
      Unfortunately, crow encountered an internal error.
    </span>
    <Accordion type="multiple" class="ml-2">
      <AccordionItem value="error" v-if="buildOutput.error !== undefined">
        <AccordionTrigger>Internal error</AccordionTrigger>
        <AccordionContent>
          <pre class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2">{{
            buildOutput.error
          }}</pre>
        </AccordionContent>
      </AccordionItem>
      <AccordionItem value="accumulatedErrors" v-if="accumulatedErrors">
        <AccordionTrigger>
          <span>
            Errors
            <span v-if="ofWhomText" class="text-sm text-muted-foreground">
              {{ ofWhomText }}
            </span>
          </span>
        </AccordionTrigger>
        <AccordionContent>
          <pre class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2"><span
            class="block"
            v-for="(line, index) in accumulatedErrorLines"
            :class="{
              'text-red-500': line.startsWith('-'),
              'text-green-600': line.startsWith('+'),
              'text-violet-500': line.startsWith('@@'),
            }"
            :key="index">{{ line }}<br v-if="line.trim().length === 0" /></span></pre>
        </AccordionContent>
      </AccordionItem>
      <AccordionItem value="stdout" v-if="buildOutput.stdout.length > 0">
        <AccordionTrigger>
          <span>
            Stdout
            <span v-if="ofWhomText" class="text-sm text-muted-foreground">
              {{ ofWhomText }}
            </span>
          </span>
        </AccordionTrigger>
        <AccordionContent>
          <pre
            v-if="isAnsi(buildOutput.stdout)"
            v-html="asAnsi(buildOutput.stdout)"
            class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2"
          />
          <pre v-else class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2">{{
            buildOutput.stdout
          }}</pre>
        </AccordionContent>
      </AccordionItem>
      <AccordionItem value="stderr" v-if="buildOutput.stderr.length > 0">
        <AccordionTrigger>
          <span>
            Stderr
            <span v-if="ofWhomText" class="text-sm text-muted-foreground">
              {{ ofWhomText }}
            </span>
          </span>
        </AccordionTrigger>
        <AccordionContent>
          <pre
            v-html="asAnsi(buildOutput.stderr)"
            v-if="isAnsi(buildOutput.stderr)"
            class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2"
          />
          <pre v-else class="overflow-auto whitespace-pre-wrap rounded bg-accent p-2">{{
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
import { AnsiUp } from 'ansi_up'
import type { ExecutionOutput } from '@/types.ts'
import { formatDuration } from '@/lib/utils.ts'

const props = defineProps<{
  output: ExecutionOutput
  subject: string
  // Whose stdout/stderr it is
  ofWhom: 'reference' | 'yours'
}>()

const { ofWhom, output, subject } = toRefs(props)

const buildOutput = computed(() => getBuildOutput(output.value))

const accumulatedErrorLines = computed(() => {
  return accumulatedErrors.value?.split('\n') || []
})

const ofWhomText = computed(() => {
  if (ofWhom.value === 'reference') {
    return ' for the reference compiler'
  }
  if (ofWhom.value === 'yours') {
    return ' for your compiler'
  }
  return null
})

const accumulatedErrors = computed(() => {
  if (output.value.type !== 'Failure') {
    return null
  }
  return output.value.accumulatedErrors
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
  if (task.type === 'Failure') {
    return task.execution
  }
  return task
}

function isAnsi(input: string): boolean {
  return input.includes('\x1b[')
}

function asAnsi(input: string): string {
  const ansi = new AnsiUp()
  ansi.escape_html = true // just to make sure they don't change the defaults
  return ansi.ansi_to_html(input)
}
</script>
