<template>
  <div
    class="flex h-[2em] w-[2em] cursor-pointer items-center justify-center rounded text-white"
    :class="[statusColor(testType(test), 'bg')]"
    v-bind="$attrs"
    @click.prevent="'output' in test ? handleTestClick(test) : undefined"
    :title="tooltip"
  >
    <LucideCheck v-if="testType(test) === 'Success'" />
    <LucideX v-else-if="testType(test) === 'Failure'" />
    <LucideFlame v-else-if="testType(test) === 'Error'" />
    <LucideUnplug v-else-if="testType(test) === 'Aborted'" />
    <LucideClockAlert v-else-if="testType(test) === 'Timeout'" />
    <span v-else-if="testType(test) === 'Queued'" />
    <RocketIcon class="animate-pulse" v-else-if="testType(test) === 'Started'" />
  </div>
</template>

<script setup lang="ts">
import { type ExecutingTest, type FinishedTestSummary } from '@/types.ts'
import { LucideCheck, LucideClockAlert, LucideFlame, LucideUnplug, LucideX } from 'lucide-vue-next'
import { computed, toRefs } from 'vue'
import { RocketIcon } from '@radix-icons/vue'
import { statusColor } from '@/lib/utils.ts'

const props = defineProps<{
  test: FinishedTestSummary | ExecutingTest
  isFinished?: boolean
}>()

const { test } = toRefs(props)

const tooltip = computed<string>(() => {
  let text = test.value.testId
  if (props.isFinished) {
    text += '\nClick the test square to see more details'
  } else {
    text += '\nWait for the run to finish to view more details'
  }
  return text
})

const emit = defineEmits<{
  testClicked: [test: FinishedTestSummary]
}>()

function testType(test: FinishedTestSummary | ExecutingTest) {
  return 'output' in test ? test.output : test.status
}

const handleTestClick = (test: FinishedTestSummary) => {
  emit('testClicked', test)
}
</script>
