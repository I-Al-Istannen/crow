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
        class="cursor-pointer hover:bg-accent"
        @click.left="openDetails(item, false)"
        @click.middle="openDetails(item, true)"
        @click.ctrl.left.capture.stop="openDetails(item, true)"
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
import { toRefs } from 'vue'
import { useRouter } from 'vue-router'
import { useTimestamp } from '@vueuse/core'

const props = defineProps<{
  queue: WorkItem[]
  runners: Runner[]
}>()
const { queue, runners } = toRefs(props)

const router = useRouter()

const currentTime = useTimestamp({ interval: 2500 })

function getRunner(runners: Runner[], taskId: TaskId) {
  return runners.find((it) => it.workingOn?.id === taskId)
}

// Sadly table rows can not be wrapped in `<a>` tags, so we need to emulate links using JS...
function openDetails(item: WorkItem, newTab: boolean) {
  const data = { name: 'task-detail', params: { taskId: item.id } }
  if (!newTab) {
    router.push(data)
  } else {
    window.open(router.resolve(data).href, '_blank')
  }
}
</script>
