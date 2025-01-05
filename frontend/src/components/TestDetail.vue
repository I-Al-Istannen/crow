<template>
  <div v-auto-animate>
    <span v-if="isFetching">Loading test data...</span>
    <span v-if="isFetched && test === null">Test not found</span>
    <div v-if="test && !isFetching" class="border p-2 mx-2 rounded">
      <div class="font-medium mb-2">Expected output</div>
      <pre class="whitespace-pre-wrap bg-accent p-2 rounded overflow-auto ml-2">{{
        test.expectedOutput
      }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TestId } from '@/types.ts'
import { queryTest } from '@/data/network.ts'
import { toRefs } from 'vue'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const props = defineProps<{
  testId: TestId
}>()
const { testId } = toRefs(props)

const { data: test, isFetched, isFetching } = queryTest(testId.value)
</script>
