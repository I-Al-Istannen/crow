<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Queue</CardTitle>
        <CardDescription>Everything you are waiting for</CardDescription>
      </CardHeader>
      <CardContent v-auto-animate>
        <div v-if="isLoading">Loading data...</div>
        <div v-if="!queueResponse && isFetched">Loading failed</div>
        <div v-if="queueResponse !== undefined">
          <div>
            <div
              class="p-2 leading-none tracking-tight inline-block"
              :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
              v-for="runner in queueResponse.runners"
              :key="runner.id"
            >
              <span class="flex flex-col justify-center">
                <span class="mb-1 font-medium">
                  {{ runner.id }}
                  <span class="ml-4 text-sm text-muted-foreground">
                    pinged {{ formatApproxDuration(currentTime, runner.lastSeen.getTime()) }} ago
                  </span>
                </span>
                <span class="text-sm text-muted-foreground flex justify-between">
                  <span>{{ runner.info }}</span>
                  <span v-if="runner.workingOn" class="font-medium">active</span>
                  <span v-else>idle</span>
                </span>
              </span>
            </div>
          </div>
          <QueuedTasksOverview :queue="queueResponse.queue" :runners="queueResponse.runners" />
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import PageContainer from '@/components/PageContainer.vue'
import QueuedTasksOverview from '@/components/QueuedTasksOverview.vue'
import { formatApproxDuration } from '../lib/utils.ts'
import { queryQueue } from '@/data/network.ts'
import { useTimestamp } from '@vueuse/core'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const currentTime = useTimestamp({ interval: 500 })

const { data: queueResponse, isFetched, isLoading } = queryQueue()
</script>
