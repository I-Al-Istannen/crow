<template>
  <Dialog v-model:open="dialogOpen">
    <DialogContent v-if="test" class="max-h-[90dvh] max-w-[80dvw] overflow-scroll">
      <DialogHeader>
        <DialogTitle>
          Test details for <span class="italic">{{ test.testId }}</span>
        </DialogTitle>
        <DialogDescription>
          <span :class="[statusColor(test.output.type, 'text')]">{{ test.output.type }}</span>
        </DialogDescription>
      </DialogHeader>
      <div>
        <ProcessOutputDisplay :output="test.output" subject="The test" />
      </div>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { FinishedTest } from '@/types.ts'
import ProcessOutputDisplay from '@/components/ProcessOutputDisplay.vue'
import { statusColor } from '@/lib/utils.ts'
import { toRefs } from 'vue'

const dialogOpen = defineModel<boolean>('dialogOpen')

const props = defineProps<{
  test?: FinishedTest
}>()

const { test } = toRefs(props)
</script>
