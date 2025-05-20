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
    <SnapshotState />
    <RerunSubmissions />
    <RehashTests />
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import RehashTests from '@/components/admin/RehashTests.vue'
import RerunSubmissions from '@/components/admin/RerunSubmissions.vue'
import SnapshotState from '@/components/admin/SnapshotState.vue'
import UserTable from '@/components/admin/UserTable.vue'
import { queryUsers } from '@/data/network.ts'

const { data: users, failureCount, failureReason, isLoading } = queryUsers()
</script>
