<template>
  <Card>
    <CardHeader>
      <CardTitle>Test results</CardTitle>
      <CardDescription>Information about individual tests</CardDescription>
    </CardHeader>
    <CardContent class="-mt-2" v-if="sortedTests.length === 0">
      No tests were run during this task.
    </CardContent>
    <CardContent class="flex flex-row gap-1 flex-wrap" v-else>
      <FinishedTestDetailDialog
        :test="clickedTest"
        :of-whom="ofWhom"
        v-model:dialog-open="dialogOpen"
      />
      <FinishedTestcaseIcon
        v-for="test in sortedTests"
        :key="test.testId"
        :test="test"
        @test-clicked="handleTestClick"
        :open-delay="0"
      />
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { ExecutingTest, FinishedTest } from '@/types.ts'
import { computed, ref, toRefs } from 'vue'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import FinishedTestcaseIcon from '@/components/FinishedTestcaseSummaryIcon.vue'

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)

const props = defineProps<{
  tests: (FinishedTest | ExecutingTest)[]
  ofWhom: 'reference' | 'yours'
}>()

const { ofWhom, tests } = toRefs(props)

const sortedTests = computed(() =>
  tests.value.slice().sort((a, b) => a.testId.localeCompare(b.testId)),
)

const handleTestClick = (test: FinishedTest) => {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
