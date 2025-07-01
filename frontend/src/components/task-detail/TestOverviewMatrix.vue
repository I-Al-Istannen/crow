<template>
  <div class="-mt-2" v-if="tests.length === 0">No tests were run during this task.</div>
  <div class="flex flex-row gap-1 flex-wrap" v-else>
    <TooltipProvider>
      <FinishedTestcaseIcon
        v-for="test in tests"
        v-memo="[test.testId, testType(test)]"
        :key="test.testId"
        :test="test"
        @test-clicked="emit('testClicked', $event)"
        :is-finished="isFinished"
      />
    </TooltipProvider>
  </div>
</template>

<script setup lang="ts">
import { type ExecutingTest, type FinishedTestSummary } from '@/types.ts'
import FinishedTestcaseIcon from '@/components/task-detail/FinishedTestcaseSummaryIcon.vue'
import { TooltipProvider } from '@/components/ui/tooltip'
import { toRefs } from 'vue'

const props = defineProps<{
  tests: (FinishedTestSummary | ExecutingTest)[]
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
