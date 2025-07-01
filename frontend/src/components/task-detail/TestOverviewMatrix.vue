<template>
  <div class="-mt-2" v-if="tests.length === 0">No tests were run during this task.</div>
  <div class="flex flex-row gap-1 flex-wrap" v-else>
    <TooltipProvider>
      <FinishedTestcaseIcon
        v-for="test in tests"
        v-memo="[testType(toValue(test))]"
        :key="toValue(test).testId"
        :test="toValue(test)"
        @test-clicked="emit('testClicked', $event)"
        :is-finished="isFinished"
      />
    </TooltipProvider>
  </div>
</template>

<script setup lang="ts">
import { type ExecutingTest, type FinishedTestSummary } from '@/types.ts'
import { toRefs, toValue } from 'vue'
import FinishedTestcaseIcon from '@/components/task-detail/FinishedTestcaseSummaryIcon.vue'
import type { MaybeRef } from '@vueuse/core'
import { TooltipProvider } from '@/components/ui/tooltip'

const props = defineProps<{
  tests: MaybeRef<FinishedTestSummary | ExecutingTest>[]
  isFinished?: boolean
}>()

const { tests } = toRefs(props)

const emit = defineEmits<{
  testClicked: [test: FinishedTestSummary]
}>()

function testType(test: FinishedTestSummary | ExecutingTest) {
  return 'output' in test ? test.output : test.status
}
</script>
