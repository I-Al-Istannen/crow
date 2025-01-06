<template>
  <PageContainer :class="{ 'flex-col-reverse': !!repo }" v-auto-animate>
    <Card>
      <CardHeader>
        <CardTitle>Repository settings</CardTitle>
        <CardDescription>Change your repository URL and crows behaviour</CardDescription>
      </CardHeader>
      <CardContent>
        <div v-if="isLoading">Requesting repo information...</div>
        <SetupRepo v-show="isFetched" :repo="repo" />
      </CardContent>
    </Card>
    <Card v-if="repo">
      <CardHeader>
        <CardTitle>Submit a revision</CardTitle>
        <CardDescription>Add a specific commit of your repository to the queue</CardDescription>
      </CardHeader>
      <CardContent>
        <SubmitRevision />
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import PageContainer from '@/components/PageContainer.vue'
import SetupRepo from '@/components/SetupRepo.vue'
import SubmitRevision from '@/components/SubmitRevision.vue'
import { computed } from 'vue'
import { queryRepo } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'


const { team } = storeToRefs(useUserStore())
const teamId = computed(() => team.value?.id)

const { data: repo, isFetched, isLoading } = queryRepo(teamId)
</script>
