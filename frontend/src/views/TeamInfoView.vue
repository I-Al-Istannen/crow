<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Team Overview</CardTitle>
        <CardDescription class="flex flex-wrap justify-between">
          <span>Information about a team and its members</span>
          <a
            v-if="info && info.repoUrl"
            :href="info.repoUrl"
            target="_blank"
            class="hover:underline"
          >
            {{ info.repoUrl }}
          </a>
        </CardDescription>
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
        <ul class="list-inside list-disc">
          <li v-for="member in info.members" :key="member.id">
            <UsernameDisplay :id="member.id" :display-name="member.displayName" :show-id="true" />
          </li>
        </ul>
      </CardContent>
    </Card>
    <TeamTasks v-if="teamId && isAdmin" :teamId="teamId" :repoUrl="info?.repoUrl || undefined" />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import type { TeamId } from '@/types.ts'
import TeamTasks from '@/components/admin/TeamTasks.vue'
import UsernameDisplay from '@/components/team/UsernameDisplay.vue'
import { computed } from 'vue'
import { queryTeamInfo } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const route = useRoute()
const teamId = computed(() => (route.params.teamId ? (route.params.teamId as TeamId) : undefined))
const { isAdmin } = storeToRefs(useUserStore())

const { data: info, isLoading, failureCount, failureReason } = queryTeamInfo(teamId)
</script>
