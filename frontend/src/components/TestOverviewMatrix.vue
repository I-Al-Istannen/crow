<template>
  <div class="-mt-2" v-if="sortedTests.length === 0">No tests were run during this task.</div>
  <div class="flex flex-row gap-1 flex-wrap" v-else>
    <FinishedTestcaseIcon
      v-for="test in sortedTests"
      :key="test.testId"
      :test="test"
      @test-clicked="emit('testClicked', $event)"
      :open-delay="0"
    />
  </div>
</template>

<script setup lang="ts">
import type { ExecutingTest, FinishedTest } from '@/types.ts'
import { computed, toRefs } from 'vue'
import FinishedTestcaseIcon from '@/components/FinishedTestcaseSummaryIcon.vue'

const props = defineProps<{
  tests: (FinishedTest | ExecutingTest)[]
}>()

const { tests } = toRefs(props)

const emit = defineEmits<{
  testClicked: [test: FinishedTest]
}>()

const sortedTests = computed(() =>
  tests.value.slice().sort((a, b) => a.testId.localeCompare(b.testId)),
)
</script>
