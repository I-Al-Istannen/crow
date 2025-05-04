<template>
  <Table v-if="queue.length > 0">
    <TableHeader>
      <TableHead class="text-center">Position</TableHead>
      <TableHead>Commit</TableHead>
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
        <TableCell class="font-medium text-center">
          {{ index + 1 }}
        </TableCell>
        <TableCell>
          {{ item.commitMessage.substring(0, 60) }}
        </TableCell>
        <TableCell>
          {{ item.revision.substring(0, 8) }}
        </TableCell>
        <TableCell>
          {{ item.team }}
        </TableCell>
        <TableCell>
          {{ formatTime(item.insertTime) }}
        </TableCell>
        <TableCell>
          {{ formatBusyDuration(currentTime, item.insertTime.getTime()) }}
        </TableCell>
        <TableCell>
          <span v-if="getRunner(runners, item.id)">{{ getRunner(runners, item.id)!.id }}</span>
          <span v-else>-</span>
        </TableCell>
      </TableRow>
    </TableBody>
  </Table>
  <div v-else class="text-muted-foreground text-sm mb-2" :class="{ 'mt-6': runners.length > 0 }">
    The queue is empty! The perfect time for you to submit something :)
  </div>
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
import { formatBusyDuration, formatTime } from '@/lib/utils.ts'
import { toRefs } from 'vue'
import { useRouter } from 'vue-router'

const props = defineProps<{
  queue: WorkItem[]
  runners: Runner[]
  currentTime: number
}>()
const { queue, runners, currentTime } = toRefs(props)

const router = useRouter()

function getRunner(runners: Runner[], taskId: TaskId) {
  return runners.find((it) => it.workingOn?.type === 'Testing' && it.workingOn?.id === taskId)
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
