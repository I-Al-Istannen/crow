<template>
  <Tooltip>
    <TooltipTrigger
      class="w-[2em] h-[2em] flex justify-center items-center text-white rounded cursor-pointer"
      :class="[statusColor(testType(test), 'bg')]"
      v-bind="$attrs"
      @click.prevent="'output' in test ? handleTestClick(test) : undefined"
    >
      <LucideCheck v-if="testType(test) === 'Success'" />
      <LucideX v-else-if="testType(test) === 'Failure'" />
      <LucideFlame v-else-if="testType(test) === 'Error'" />
      <LucideUnplug v-else-if="testType(test) === 'Aborted'" />
      <LucideClockAlert v-else-if="testType(test) === 'Timeout'" />
      <span v-else-if="testType(test) === 'Queued'" />
      <RocketIcon class="animate-pulse" v-else-if="testType(test) === 'Started'" />
    </TooltipTrigger>
    <TooltipContent class="w-96 text-sm">
      <span class="font-medium"> {{ test.testId }} </span>:
      <span :class="[statusColor(testType(test), 'text')]">{{ testType(test) }}</span>
      <br />
      <span class="text-sm text-muted-foreground" v-if="isFinished">
        Click the test square to see more details
      </span>
      <span class="text-sm text-muted-foreground" v-else>
        Wait for the run to finish to view more details
      </span>
    </TooltipContent>
  </Tooltip>
</template>

<script setup lang="ts">
import { type ExecutingTest, type FinishedTest, toExecutionStatus } from '@/types.ts'
import { LucideCheck, LucideClockAlert, LucideFlame, LucideUnplug, LucideX } from 'lucide-vue-next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { RocketIcon } from '@radix-icons/vue'
import { statusColor } from '@/lib/utils.ts'
import { toRefs } from 'vue'

const props = defineProps<{
  test: FinishedTest | ExecutingTest
  isFinished?: boolean
}>()

const { test } = toRefs(props)

const emit = defineEmits<{
  testClicked: [test: FinishedTest]
}>()

function testType(test: FinishedTest | ExecutingTest) {
  return 'output' in test ? toExecutionStatus(test.output) : test.status
}

const handleTestClick = (test: FinishedTest) => {
  emit('testClicked', test)
}
</script>
