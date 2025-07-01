<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row items-center justify-between">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>Queue</CardTitle>
          <CardDescription>Everything you are waiting for</CardDescription>
        </div>
        <div class="mr-2">
          <span v-if="!isFetching && nextRefetchTime">{{ nextRefetchTime }}</span>
          <LucideLoaderCircle v-if="isFetching" class="animate-spin" />
        </div>
      </CardHeader>
      <CardContent v-auto-animate>
        <DataLoadingExplanation
          :is-loading="isLoading"
          :failure-count="failureCount"
          :failure-reason="failureReason"
        />
        <div v-if="queueResponse">
          <TooltipProvider>
            <div class="mb-4 flex flex-wrap gap-2">
              <div
                class="flex flex-row items-center gap-2 p-2 leading-none tracking-tight"
                :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
                v-for="runner in sortedRunners"
                :key="runner.id"
              >
                <Tooltip v-if="runner.testTaster">
                  <TooltipTrigger as-child>
                    <LucideCandy :size="16" />
                  </TooltipTrigger>
                  <TooltipContent>This runner focuses solely on test tasting</TooltipContent>
                </Tooltip>
                <Tooltip v-else>
                  <TooltipTrigger as-child>
                    <LucideBriefcaseBusiness :size="16" />
                  </TooltipTrigger>
                  <TooltipContent>This runner runs tests against your submissions</TooltipContent>
                </Tooltip>
                <div class="flex flex-col justify-center">
                  <div class="mb-1 flex items-center justify-between gap-2 font-medium">
                    <span>
                      {{ runner.id }}
                    </span>
                    <span class="text-sm text-muted-foreground sm:ml-4">
                      pinged
                      <span class="inline-block min-w-[3ch] text-end">
                        {{ formatApproxDuration(currentTime, runner.lastSeen.getTime()) }}
                      </span>
                      ago
                    </span>
                  </div>
                  <span class="flex justify-between text-sm text-muted-foreground">
                    <span>{{ runner.info }}</span>
                    <span v-if="runner.workingOn" class="gradient-primary font-medium">
                      active
                    </span>
                    <span v-else>idle</span>
                  </span>
                </div>
              </div>
            </div>
          </TooltipProvider>
          <QueuedTasksOverview
            :queue="queueResponse.queue"
            :runners="queueResponse.runners"
            :current-time="currentTime"
          />
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { LucideBriefcaseBusiness, LucideCandy, LucideLoaderCircle } from 'lucide-vue-next'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { computed, ref, watch } from 'vue'
import { formatApproxDuration, formatDuration } from '../lib/utils.ts'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import QueuedTasksOverview from '@/components/queue/QueuedTasksOverview.vue'
import { queryQueue } from '@/data/network.ts'
import { useTimestamp } from '@vueuse/core'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const currentTime = useTimestamp({ interval: 500 })
const nextRefetch = ref(Date.now())

const {
  data: queueResponse,
  isLoading,
  isFetching,
  failureCount,
  failureReason,
} = queryQueue(15 * 1000)

const nextRefetchTime = computed(() => {
  const delta = nextRefetch.value - currentTime.value
  if (delta < 1000) {
    return '0s'
  }
  return formatDuration(delta)
})

const sortedRunners = computed(() => {
  if (!queueResponse.value) {
    return []
  }
  return queueResponse.value.runners.slice().sort((a, b) => {
    if (a.testTaster && !b.testTaster) {
      return 1
    }
    if (!a.testTaster && b.testTaster) {
      return -1
    }
    return a.id.localeCompare(b.id)
  })
})

watch(isFetching, (val) => {
  if (!val) {
    nextRefetch.value = Date.now() + 15 * 1000
  }
})
</script>
