<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Queue</CardTitle>
        <CardDescription>Everything you are waiting for</CardDescription>
      </CardHeader>
      <CardContent v-auto-animate>
        <div v-if="isLoading">Loading data...</div>
        <div v-if="!queue && isFetched">Loading failed</div>
        <div v-if="queue !== undefined" class="space-y-2">
          <Table>
            <TableHeader>
              <TableHead>Position</TableHead>
              <TableHead>Revision</TableHead>
              <TableHead>Team</TableHead>
              <TableHead>Queued since</TableHead>
              <TableHead>Queued for</TableHead>
            </TableHeader>
            <TableBody>
              <TableRow v-for="(item, index) in queue" :key="item.id">
                <TableCell class="font-medium">
                  {{ index + 1 }}
                </TableCell>
                <TableCell>
                  {{ item.revision }}
                </TableCell>
                <TableCell>
                  {{ item.team }}
                </TableCell>
                <TableCell>
                  {{ formatTime(item.insertTime) }}
                </TableCell>
                <TableCell>
                  {{ formatApproxDuration(currentTime, item.insertTime.getTime()) }}
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { formatDuration, formatTime } from '../lib/utils.ts'
import PageContainer from '@/components/PageContainer.vue'
import { queryQueue } from '@/data/network.ts'
import { useTimestamp } from '@vueuse/core'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const currentTime = useTimestamp({ interval: 2500 })

const { data: queue, isFetched, isLoading } = queryQueue()

function formatApproxDuration(currentTime: number, insertTime: number) {
  return formatDuration(Math.floor((currentTime - insertTime) / 5000) * 5000)
}
</script>
