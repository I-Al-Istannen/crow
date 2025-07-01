<template>
  <TooltipProvider v-if="finalizedForCategory && finalizedForCategory.length > 0">
    <Tooltip>
      <TooltipTrigger class="flex items-center">
        <LucideSend class="inline-block h-4 w-4 cursor-default text-blue-400" />
      </TooltipTrigger>
      <TooltipContent>
        <span class="text-white">crow</span> has locked in this task as your submission for
        {{ finalizedForCategory.join(' and ') }}.
        <br />
        It was either the task you manually selected or the best task according to
        <span class="text-white">crow</span>'s heuristics at the time of submission.
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>

<script setup lang="ts">
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { computed, toRefs } from 'vue'
import { LucideSend } from 'lucide-vue-next'
import type { TaskId } from '@/types.ts'
import { queryFinalSubmittedTasks } from '@/data/network.ts'

const props = defineProps<{
  taskId: TaskId
}>()
const { taskId } = toRefs(props)

const { data: gradedTasks } = queryFinalSubmittedTasks()

const finalizedForCategory = computed(() => {
  if (!gradedTasks.value) {
    return false
  }
  return Array.from(gradedTasks.value.entries())
    .filter(
      ([_cat, gradedTask]) =>
        gradedTask.summary.info.taskId === taskId.value && gradedTask.type === 'Finalized',
    )
    .map(([category]) => category)
})
</script>
