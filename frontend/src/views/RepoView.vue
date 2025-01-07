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
        <Tabs default-value="manual">
          <TabsList>
            <TabsTrigger value="manual">Manually</TabsTrigger>
            <TabsTrigger value="github" v-if="integrationStatus?.github">GitHub App</TabsTrigger>
            <TabsTrigger value="token">Build your own integration</TabsTrigger>
          </TabsList>
          <TabsContent value="manual">
            <SubmitRevision />
          </TabsContent>
          <TabsContent value="github" v-if="integrationStatus?.github">
            <TeamIntegrationGithub :app-url="integrationStatus?.github.url" />
          </TabsContent>
          <TabsContent value="token">
            <TeamIntegrationToken
              v-if="integrationStatus"
              :team-integration-token="integrationStatus?.token"
            />
            <span v-else class="text-muted-foreground text-sm">Not in a team</span>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { queryIntegrationStatus, queryRepo } from '@/data/network.ts'
import PageContainer from '@/components/PageContainer.vue'
import SetupRepo from '@/components/SetupRepo.vue'
import SubmitRevision from '@/components/SubmitRevision.vue'
import TeamIntegrationGithub from '@/components/TeamIntegrationGithub.vue'
import TeamIntegrationToken from '@/components/TeamIntegrationToken.vue'
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const { team } = storeToRefs(useUserStore())
const teamId = computed(() => team.value?.id)

const { data: repo, isFetched, isLoading } = queryRepo(teamId)

const { data: integrationStatus } = queryIntegrationStatus(teamId)
</script>
