<template>
  <Table>
    <TableHeader>
      <TableHead>Position</TableHead>
      <TableHead>Revision</TableHead>
      <TableHead>Team</TableHead>
      <TableHead>Queued since</TableHead>
      <TableHead>Queued for</TableHead>
      <TableHead>Running on</TableHead>
    </TableHeader>
    <TableBody>
      <TableRow
        v-for="(item, index) in queue"
        :key="item.id"
        :class="[
          team?.id == item.team
            ? 'animate-gradient-x bg-gradient-to-r from-blue-500 via-violet-500 to-rose-600 bg-clip-text text-transparent'
            : '',
        ]"
      >
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
        <TableCell>
          <span v-if="getRunner(runners, item.id)">{{ getRunner(runners, item.id)!.id }}</span>
          <span v-else>-</span>
        </TableCell>
      </TableRow>
    </TableBody>
  </Table>
</template>

<script setup lang="ts">
import type { Runner, TaskId, WorkItem } from '@/types.ts'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { formatApproxDuration, formatTime } from '@/lib/utils.ts'
import { storeToRefs } from 'pinia'
import { toRefs } from 'vue'
import { useTimestamp } from '@vueuse/core'
import { useUserStore } from '@/stores/user.ts'

const props = defineProps<{
  queue: WorkItem[]
  runners: Runner[]
}>()
const { queue, runners } = toRefs(props)

const { team } = storeToRefs(useUserStore())
const currentTime = useTimestamp({ interval: 2500 })

function getRunner(runners: Runner[], taskId: TaskId) {
  return runners.find((it) => it.workingOn?.id === taskId)
}
</script>
