<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Team Overview</CardTitle>
        <CardDescription>Information about a team and its members</CardDescription>
      </CardHeader>
      <CardContent v-if="info">
        <DataLoadingExplanation
          :is-loading="isLoading"
          :failure-count="failureCount"
          :failure-reason="failureReason"
        />
        <span class="font-medium">{{ info.team.displayName }}</span>
        <span class="text-muted-foreground"> ({{ info.team.id }})</span>
        consists of
        <ul class="list-disc list-inside">
          <li v-for="member in info.members" :key="member.id">
            <UsernameDisplay :id="member.id" :display-name="member.displayName" :show-id="true" />
          </li>
        </ul>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import type { TeamId } from '@/types.ts'
import UsernameDisplay from '@/components/UsernameDisplay.vue'
import { computed } from 'vue'
import { queryTeamInfo } from '@/data/network.ts'
import { useRoute } from 'vue-router'

const route = useRoute()
const teamId = computed(() => (route.params.teamId ? (route.params.teamId as TeamId) : undefined))

const { data: info, isLoading, failureCount, failureReason } = queryTeamInfo(teamId)
</script>
