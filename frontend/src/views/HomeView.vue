<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row justify-between items-center">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>Recent runs</CardTitle>
          <CardDescription>View your recent submissions</CardDescription>
        </div>
        <div class="mr-2 hover:underline">
          <RouterLink :to="{ name: 'all-tasks' }">View all results</RouterLink>
        </div>
      </CardHeader>
      <CardContent class="space-y-1" v-if="isFetched && data">
        <FinishedTaskOverview v-for="task in data" :task="task" :key="task.info.taskId" />
        <div v-if="data.length === 0" class="text-muted-foreground text-sm mb-2">
          No tasks here yet :/ You need to submit some (or set up your repository :)
        </div>
      </CardContent>
      <CardContent v-if="isLoading">Loading recent tasks...</CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import FinishedTaskOverview from '@/components/FinishedTaskOverview.vue'
import PageContainer from '@/components/PageContainer.vue'
import { queryRecentTasks } from '@/data/network.ts'

const { data, isFetched, isLoading } = queryRecentTasks()
</script>
