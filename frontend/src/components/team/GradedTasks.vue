<template>
  <Card v-if="isLoading || (sortedTasks?.length || 0) > 0">
    <CardHeader>
      <CardTitle>Tasks for grading</CardTitle>
      <CardDescription>
        The currently submitted tasks for each category. Tasks marked with a star were automatically
        selected by crow, hover it for more info.
      </CardDescription>
    </CardHeader>
    <CardContent v-auto-animate>
      <DataLoadingExplanation
        :is-loading="isLoading"
        :failure-count="failureCount"
        :failure-reason="failureReason"
      />
      <div v-if="tasks" class="-mt-2 space-y-2">
        <TooltipProvider>
          <div v-for="[category, task] in sortedTasks" :key="category" class="flex flex-col">
            <span class="mb-1 font-medium">
              {{ category }}:
              <Tooltip v-if="task.type === 'Finalized' && task.points">
                <TooltipTrigger as-child>
                  <span class="ml-2 text-sm text-muted-foreground">
                    You got {{ task.points.points }} points, hover for details.
                    <span v-if="task.points.points >= 80">
                      <span class="gradient-primary">Perfect!</span> 🐞</span
                    >
                    <span v-else-if="task.points.points >= 60" class="text-gray-700">
                      Nice one!
                    </span>
                    <span v-else-if="task.points.points >= 40" class="text-gray-700">
                      Well done!
                    </span>
                    <span v-else-if="task.points.points >= 20" class="text-gray-700">
                      Good start, keep going!
                    </span>
                    <span v-else class="text-gray-700">You can do better, I believe in you :)</span>
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  The rating formula for this lab is
                  <span class="ml-1 font-mono">{{ task.points.formula }}</span>
                </TooltipContent>
              </Tooltip>
            </span>
            <FinishedTaskOverview :task="task.summary" class="ml-2" />
          </div>
        </TooltipProvider>
      </div>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import { computed } from 'vue'
import { queryFinalSubmittedTasks } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const { data: tasks, isLoading, failureCount, failureReason } = queryFinalSubmittedTasks()

const sortedTasks = computed(() => {
  if (!tasks.value) {
    return undefined
  }
  const result = Array.from(tasks.value.entries())
  result.sort((a, b) => b[0].localeCompare(a[0]))

  return result
})
</script>
