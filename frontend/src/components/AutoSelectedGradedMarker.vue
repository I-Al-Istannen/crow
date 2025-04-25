<template>
  <TooltipProvider v-if="gradedForCategory && gradedForCategory.length > 0">
    <Tooltip>
      <TooltipTrigger>
        <LucideSparkles class="h-4 w-4 inline-block text-yellow-500"></LucideSparkles>
      </TooltipTrigger>
      <TooltipContent>
        Automatically submitting this task for {{ gradedForCategory.join(' and ') }}, as it
        currently passes the most tests.
        <br />
        This will automatically update, you do not need to do anything.
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>

<script setup lang="ts">
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { computed, toRefs } from 'vue'
import { LucideSparkles } from 'lucide-vue-next'
import type { TaskId } from '@/types.ts'
import { queryFinalSubmittedTasks } from '@/data/network.ts'

const props = defineProps<{
  taskId: TaskId
}>()
const { taskId } = toRefs(props)

const { data: gradedTasks } = queryFinalSubmittedTasks()

const gradedForCategory = computed(() => {
  if (!gradedTasks.value) {
    return false
  }
  return Array.from(gradedTasks.value.entries())
    .filter(
      ([_cat, gradedTask]) =>
        gradedTask.summary.info.taskId === taskId.value &&
        gradedTask.type === 'AutomaticallySelected',
    )
    .map(([category]) => category)
})
</script>
