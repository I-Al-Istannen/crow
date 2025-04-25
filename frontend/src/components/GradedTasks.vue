<template>
  <Card v-if="isLoading || (sortedTasks?.length || 0) > 0">
    <CardHeader>
      <CardTitle>Tasks for grading</CardTitle>
      <CardDescription>
        Your best runs for each category, automatically cherry picked. You do not need to do
        anything.
      </CardDescription>
    </CardHeader>
    <CardContent v-auto-animate>
      <DataLoadingExplanation
        :is-loading="isLoading"
        :failure-count="failureCount"
        :failure-reason="failureReason"
      />
      <div v-if="tasks" class="space-y-2 -mt-2">
        <div v-for="[category, task] in sortedTasks" :key="category" class="flex flex-col">
          <span class="font-medium mb-1">{{ category }}:</span>
          <FinishedTaskOverview :task="task.summary" class="ml-2" />
        </div>
      </div>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import FinishedTaskOverview from '@/components/FinishedTaskOverview.vue'
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
