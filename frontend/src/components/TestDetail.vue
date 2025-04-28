<template>
  <div>
    <span v-if="isLoading">Loading test data...</span>
    <span v-if="isFetched && test === null">Test not found</span>
    <div v-if="test" class="border p-2 mx-2 rounded flex flex-col gap-2">
      <div v-if="testTastingError">
        <div class="font-medium mb-2">Test tasting</div>
        <FinishedTestDetailDialog
          :test="testTastingError"
          of-whom="reference"
          v-model:dialog-open="failedTastingDialogOpen"
        />
        <FinishedTestcaseSummaryIcon
          :test="testTastingError"
          class="ml-2"
          @test-clicked="failedTastingDialogOpen = true"
        />
      </div>
      <div v-if="test" class="grid grid-cols-1 lg:grid-cols-2 gap-4 p-1">
        <div>
          <span class="text-sm font-medium">Executing your compiler</span>
          <TestModifierList
            :value="test.compilerModifiers.map((val, key) => ({ ...val, key }))"
            modifier-target="compiler"
            readonly
          />
        </div>
        <div>
          <span class="text-sm font-medium">Executing the compiled binary</span>
          <TestModifierList
            :value="test.binaryModifiers.map((val, key) => ({ ...val, key }))"
            modifier-target="binary"
            readonly
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { FinishedTest, TestId } from '@/types.ts'
import { computed, ref, toRefs } from 'vue'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import FinishedTestcaseSummaryIcon from '@/components/FinishedTestcaseSummaryIcon.vue'
import TestModifierList from '@/components/ui/TestModifierList.vue'
import { queryTest } from '@/data/network.ts'

const failedTastingDialogOpen = ref<boolean>(false)

const props = defineProps<{
  testId: TestId
}>()
const { testId } = toRefs(props)

const { data: test, isFetched, isLoading } = queryTest(testId.value)

const testTastingError = computed<FinishedTest | null>(() => {
  if (test.value?.testTastingResult?.type !== 'Failure') {
    return null
  }
  return {
    testId: testId.value,
    output: test.value.testTastingResult.output,
    provisionalForCategory: test.value.provisionalForCategory,
  }
})
</script>
