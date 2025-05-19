<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Users</CardTitle>
        <CardDescription>Information about users that logged in at least once</CardDescription>
      </CardHeader>
      <CardContent>
        <DataLoadingExplanation
          :isLoading="isLoading"
          :failureCount="failureCount"
          :failureReason="failureReason"
        />
        <UserTable v-if="users" :users="users" />
      </CardContent>
    </Card>
    <Card>
      <CardHeader>
        <CardTitle>Snapshots</CardTitle>
        <CardDescription>Create full database and repo snapshots</CardDescription>
      </CardHeader>
      <CardContent>
        <Button :disabled="snapshotPending" @click="createSnapshot">Create Snapshot</Button>
        <div class="text-destructive mt-4" v-if="snapshotError">
          Snapshot failed
          <br />
          {{ snapshotError }}
        </div>
        <ul
          class="mt-4 text-sm list-disc mx-4"
          v-if="snapshotResult && snapshotResult.errors.length > 0"
        >
          <li v-for="error in snapshotResult.errors" :key="error">
            <pre class="text-destructive">{{ error }}</pre>
          </li>
          <li v-for="team in snapshotResult.exported" :key="team">
            <span class="text-muted-foreground">Backed up</span> {{ team }}
          </li>
        </ul>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { mutateCreateSnapshot, queryUsers } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import UserTable from '@/components/admin/UserTable.vue'

const { data: users, failureCount, failureReason, isLoading } = queryUsers()

const {
  mutateAsync: snapshot,
  isPending: snapshotPending,
  error: snapshotError,
  data: snapshotResult,
} = mutateCreateSnapshot()

async function createSnapshot() {
  await snapshot()
}
</script>
