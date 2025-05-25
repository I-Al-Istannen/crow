<template>
  <Card>
    <CardHeader>
      <CardTitle>Team Statistics</CardTitle>
      <CardDescription> Statistics about teams, grouped by categories.</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <DataLoadingExplanation
        :isLoading="statIsLoading"
        :failureCount="statFailureCount"
        :failureReason="statFailureReason"
      />
      <template v-if="teamStatistics">
        <TeamStatisticForCategory
          v-for="category in categories"
          :key="category"
          :category="category"
          :statistics="teamStatistics"
        />
      </template>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import TeamStatisticForCategory from '@/components/admin/TeamStatisticForCategory.vue'
import { computed } from 'vue'
import { queryTeamStatistics } from '@/data/network.ts'

const {
  data: teamStatistics,
  failureCount: statFailureCount,
  failureReason: statFailureReason,
  isLoading: statIsLoading,
} = queryTeamStatistics()

const categories = computed(() => {
  if (!teamStatistics.value) {
    return []
  }
  return Array.from(
    new Set(teamStatistics.value.flatMap((it) => Object.keys(it.testsPerCategory))),
  ).sort((a, b) => b.localeCompare(a))
})
</script>
