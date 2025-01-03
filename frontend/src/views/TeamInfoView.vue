<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Team Overview</CardTitle>
        <CardDescription>Information about a team and its members</CardDescription>
      </CardHeader>
      <CardContent v-if="isLoading"> Loading team information...</CardContent>
      <CardContent v-if="isFetched && info === null"> Team not found</CardContent>
      <CardContent v-if="isFetched && info">
        <span class="font-medium">{{ info.team.displayName }}</span>
        <span class="text-muted-foreground"> ({{ info.team.id }})</span>
        consists of
        <ul class="list-disc list-inside">
          <li v-for="member in info.members" :key="member.id">
            <span
              class="font-medium"
              :class="[
                user?.id === member.id
                  ? 'animate-gradient-x bg-gradient-to-r from-blue-500 via-violet-500 to-rose-600 bg-clip-text text-transparent'
                  : '',
              ]"
            >
              {{ member.displayName }}
            </span>
            <span class="text-muted-foreground"> ({{ member.id }})</span>
          </li>
        </ul>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import PageContainer from '@/components/PageContainer.vue'
import type { TeamId } from '@/types.ts'
import { computed } from 'vue'
import { queryTeamInfo } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

const route = useRoute()
const teamId = computed(() => (route.params.teamId ? (route.params.teamId as TeamId) : undefined))

const { user } = storeToRefs(useUserStore())

const { data: info, isFetched, isLoading } = queryTeamInfo(teamId)
</script>
