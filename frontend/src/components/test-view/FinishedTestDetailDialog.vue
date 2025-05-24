<template>
  <Dialog v-model:open="dialogOpen">
    <DialogContent v-if="test" class="max-h-[90dvh] max-w-[80dvw] overflow-scroll">
      <DialogHeader>
        <DialogTitle>
          Test details for <span class="italic">{{ test.testId }}</span>
          <span v-if="test.category" class="ml-3 text-sm text-muted-foreground">
            {{ test.category }}
          </span>
        </DialogTitle>
        <DialogDescription>
          <span :class="[statusColor(toExecutionStatus(test.output), 'text')]">
            {{ test.output.type }}
          </span>
        </DialogDescription>
      </DialogHeader>
      <div class="space-y-8">
        <ProcessOutputDisplay
          v-if="compilerOutput"
          :output="compilerOutput"
          :of-whom="ofWhom"
          subject="Compilation"
        />
        <ProcessOutputDisplay
          v-if="binaryOutput"
          :output="binaryOutput"
          :of-whom="ofWhom"
          subject="Execution"
        />
        <div v-if="!(hideTestContent === true)">
          <span class="font-semibold"> Executed Test </span>
          <Accordion type="multiple" class="ml-2">
            <AccordionItem value="test-details">
              <AccordionTrigger>Expand to view full test</AccordionTrigger>
              <AccordionContent>
                <TestDetail v-if="!outdated" :test-id="test.testId" />
                <span v-else class="ml-1 text-muted-foreground">
                  This test has since been updated and no longer reflects what was ran. Therefore, I
                  can't show you the test details. I am sorry.
                </span>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </div>
      </div>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { type FinishedTest, toBinaryOutput, toCompilerOutput, toExecutionStatus } from '@/types.ts'
import { computed, toRefs } from 'vue'
import ProcessOutputDisplay from '@/components/test-view/ProcessOutputDisplay.vue'
import TestDetail from '@/components/test-view/TestDetail.vue'
import { statusColor } from '@/lib/utils.ts'

const dialogOpen = defineModel<boolean>('dialogOpen')

const props = defineProps<{
  test?: FinishedTest
  ofWhom: 'reference' | 'yours'
  hideTestContent?: boolean
  outdated?: boolean
}>()

const { test, ofWhom, hideTestContent } = toRefs(props)

const compilerOutput = computed(() =>
  test.value ? toCompilerOutput(test.value.output) : undefined,
)
const binaryOutput = computed(() => (test.value ? toBinaryOutput(test.value.output) : undefined))
</script>
