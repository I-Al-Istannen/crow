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
          :class="[statusColor(testType(test), 'bg')]"
          @click="'output' in test ? handleTestClick(test) : undefined"
        >
          <LucideCheck v-if="testType(test) === 'Success'" />
          <LucideX v-else-if="testType(test) === 'Failure'" />
          <LucideFlame v-else-if="testType(test) === 'Error'" />
          <LucideUnplug v-else-if="testType(test) === 'Aborted'" />
          <LucideClockAlert v-else-if="testType(test) === 'Timeout'" />
          <LucideLoaderCircle
            class="animate-[spin_10s_linear_infinite]"
            v-else-if="testType(test) === 'Queued'"
          />
          <RocketIcon class="animate-pulse" v-else-if="testType(test) === 'Started'" />
        </HoverCardTrigger>
        <HoverCardContent class="w-96">
          <span class="font-medium"> {{ test.testId }} </span>:
          <span :class="[statusColor(testType(test), 'text')]">{{ testType(test) }}</span>
          <br />
          <span class="text-sm text-muted-foreground" v-if="'output' in test">
            Click the test square to see more details
          </span>
          <span class="text-sm text-muted-foreground" v-else>
            Wait for the test to finish to view more details
          </span>
        </HoverCardContent>
      </HoverCard>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { ExecutingTest, FinishedTest } from '@/types.ts'
import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card'
import {
  LucideCheck,
  LucideClockAlert,
  LucideFlame,
  LucideLoaderCircle,
  LucideUnplug,
  LucideX
} from 'lucide-vue-next'
import { computed, ref, toRefs } from 'vue'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import { RocketIcon } from '@radix-icons/vue'
import { statusColor } from '@/lib/utils.ts'

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)

const props = defineProps<{
  tests: (FinishedTest | ExecutingTest)[]
}>()

const { tests } = toRefs(props)

const sortedTests = computed(() =>
  tests.value.slice().sort((a, b) => a.testId.localeCompare(b.testId)),
)

function testType(test: FinishedTest | ExecutingTest) {
  return 'output' in test ? test.output.type : test.status
}

const handleTestClick = (test: FinishedTest) => {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
