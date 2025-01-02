<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Recent runs</CardTitle>
        <CardDescription>View your recent submissions</CardDescription>
      </CardHeader>
      <CardContent class="space-y-1">
        <FinishedTaskOverview v-for="task in data" :task="task" :key="task.info.taskId" />
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import PageContainer from '@/components/PageContainer.vue'
import { queryRecentTasks } from '@/data/network.ts'
import FinishedTaskOverview from '@/components/FinishedTaskOverview.vue'

const { data, isFetched, isFetching } = queryRecentTasks()
</script>
