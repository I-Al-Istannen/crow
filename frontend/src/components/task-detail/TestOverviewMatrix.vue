<template>
  <div class="-mt-2" v-if="tests.length === 0">No tests were run during this task.</div>
  <div class="flex flex-row gap-1 flex-wrap" v-else>
    <FinishedTestcaseIcon
      v-for="test in tests"
      v-memo="[test.testId, testType(test)]"
      :key="test.testId"
      :test="test"
      @test-clicked="emit('testClicked', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { type ExecutingTest, type FinishedTest, toExecutionStatus } from '@/types.ts'
import FinishedTestcaseIcon from '@/components/task-detail/FinishedTestcaseSummaryIcon.vue'
import { toRefs } from 'vue'

const props = defineProps<{
  tests: (FinishedTest | ExecutingTest)[]
}>()

const { tests } = toRefs(props)

const emit = defineEmits<{
  testClicked: [test: FinishedTest]
}>()

function testType(test: FinishedTest | ExecutingTest) {
  return 'output' in test ? toExecutionStatus(test.output) : test.status
}
</script>
