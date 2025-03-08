<template>
  <div v-auto-animate>
    <span v-if="isFetching">Loading test data...</span>
    <span v-if="isFetched && test === null">Test not found</span>
    <div v-if="test && !isFetching" class="border p-2 mx-2 rounded flex flex-col gap-2">
      <div v-if="testTastingError">
        <div class="font-medium mb-2">Test tasting</div>
        <FinishedTestDetailDialog
          :test="testTastingError"
          v-model:dialog-open="failedTastingDialogOpen"
        />
        <FinishedTestcaseSummaryIcon
          :test="testTastingError"
          class="ml-2"
          @test-clicked="failedTastingDialogOpen = true"
        />
      </div>
      <div>
        <div class="font-medium mb-2">Input</div>
        <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto ml-2">{{
          test.input
        }}</pre>
      </div>
      <div>
        <div class="font-medium mb-2">Expected output</div>
        <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto ml-2">{{
          test.expectedOutput
        }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { FinishedTest, TestId } from '@/types.ts'
import { computed, ref, toRefs } from 'vue'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import FinishedTestcaseSummaryIcon from '@/components/FinishedTestcaseSummaryIcon.vue'
import { queryTest } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const failedTastingDialogOpen = ref<boolean>(false)

const props = defineProps<{
  testId: TestId
}>()
const { testId } = toRefs(props)

const { data: test, isFetched, isFetching } = queryTest(testId.value)

const testTastingError = computed<FinishedTest | null>(() => {
  if (test.value?.testTastingResult?.type !== 'Failure') {
    return null
  }
  return {
    testId: testId.value,
    output: test.value.testTastingResult.output,
  }
})
</script>
