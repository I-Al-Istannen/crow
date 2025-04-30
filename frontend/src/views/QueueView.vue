<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row justify-between items-center">
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
            <div class="flex gap-2 flex-wrap">
              <div
                class="p-2 leading-none tracking-tight flex flex-row gap-2 items-center"
                :class="['rounded-xl', 'border', 'bg-card', 'text-card-foreground']"
                v-for="runner in queueResponse.runners"
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
                  <div class="mb-1 font-medium flex gap-2 items-center justify-between">
                    <span>
                      {{ runner.id }}
                    </span>
                    <span class="sm:ml-4 text-sm text-muted-foreground">
                      pinged
                      <span class="min-w-[3ch] inline-block">
                        {{ formatApproxDuration(currentTime, runner.lastSeen.getTime()) }}
                      </span>
                      ago
                    </span>
                  </div>
                  <span class="text-sm text-muted-foreground flex justify-between">
                    <span>{{ runner.info }}</span>
                    <span v-if="runner.workingOn" class="font-medium gradient-primary">
                      active
                    </span>
                    <span v-else>idle</span>
                  </span>
                </div>
              </div>
            </div>
          </TooltipProvider>
          <QueuedTasksOverview :queue="queueResponse.queue" :runners="queueResponse.runners" />
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

watch(isFetching, (val) => {
  if (!val) {
    nextRefetch.value = Date.now() + 15 * 1000
  }
})
</script>
