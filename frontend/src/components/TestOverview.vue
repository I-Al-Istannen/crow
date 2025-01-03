<template>
  <Card>
    <CardHeader>
      <CardTitle>Test results</CardTitle>
      <CardDescription>Information about individual tests</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-row gap-1 flex-wrap">
      <FinishedTestDetailDialog :test="clickedTest" v-model:dialog-open="dialogOpen" />
      <HoverCard v-for="test in sortedTests" :key="test.testId" :open-delay="0">
        <HoverCardTrigger
          class="w-[2em] h-[2em] flex justify-center items-center text-white rounded cursor-pointer"
          :class="[statusColor(test.output.type, 'bg')]"
          @click="handleTestClick(test)"
        >
          <LucideCheck v-if="test.output.type === 'Finished'" />
          <LucideX v-else-if="test.output.type === 'Error'" />
          <LucideUnplug v-else-if="test.output.type === 'Aborted'" />
          <LucideClockAlert v-else-if="test.output.type === 'Timeout'" />
        </HoverCardTrigger>
        <HoverCardContent class="w-96">
          <span class="font-medium"> {{ test.testId }} </span>:
          <span :class="[statusColor(test.output.type, 'text')]">{{ test.output.type }}</span>
          <br />
          <span class="text-sm text-muted-foreground">
            Click the test square to see more details
          </span>
        </HoverCardContent>
      </HoverCard>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card'
import { LucideCheck, LucideClockAlert, LucideUnplug, LucideX } from 'lucide-vue-next'
import { computed, ref, toRefs } from 'vue'
import type { FinishedTest } from '@/types.ts'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import { statusColor } from '@/lib/utils.ts'

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)

const props = defineProps<{
  tests: FinishedTest[]
}>()

const { tests } = toRefs(props)

const sortedTests = computed(() =>
  tests.value.slice().sort((a, b) => a.testId.localeCompare(b.testId)),
)

const handleTestClick = (test: FinishedTest) => {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
