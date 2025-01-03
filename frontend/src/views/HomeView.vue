<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Recent runs</CardTitle>
        <CardDescription>View your recent submissions</CardDescription>
      </CardHeader>
      <CardContent class="space-y-1" v-if="isFetched">
        <FinishedTaskOverview v-for="task in data" :task="task" :key="task.info.taskId" />
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
