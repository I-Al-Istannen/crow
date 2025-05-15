<template>
  <Table class="w-fit">
    <TableHeader>
      <TableHead class="text-center">Place</TableHead>
      <TableHead class="text-center">Team</TableHead>
      <TableHead class="text-center">Tests</TableHead>
      <TableHead class="text-center">Time</TableHead>
      <TableHead class="text-center">Task</TableHead>
    </TableHeader>
    <TableBody>
      <TableRow
        v-for="([teamId, task], index) in sortedTeams"
        :key="teamId"
        @click="goto(task.info.taskId, $event)"
        @click.middle="goto(task.info.taskId, $event)"
        class="cursor-pointer"
      >
        <TableCell class="py-0 text-center">{{ index + 1 }}</TableCell>
        <TableCell class="py-0">{{ task.teamName }}</TableCell>
        <TableCell class="py-0">
          <TaskQuickOverview class="text-sm" :task="task" />
        </TableCell>
        <TableCell class="py-0">{{ task.info.start.toLocaleString() }}</TableCell>
        <TableCell class="py-0">
          <RouterLink
            :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }"
            @click.capture.stop="null"
            @click.middle.capture.stop="null"
          >
            <Button variant="link">To the task</Button>
          </RouterLink>
        </TableCell>
      </TableRow>
    </TableBody>
  </Table>
</template>

<script setup lang="ts">
import type { ApiFinishedCompilerTaskSummary, TaskId, TeamId } from '@/types.ts'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import TaskQuickOverview from '@/components/task-overview/TaskQuickOverview.vue'
import { useRouter } from 'vue-router'

defineProps<{
  sortedTeams: [TeamId, ApiFinishedCompilerTaskSummary][]
}>()

const router = useRouter()

function goto(taskId: TaskId, event: MouseEvent) {
  if (event.ctrlKey || event.button === 1) {
    window.open(router.resolve({ name: 'task-detail', params: { taskId } }).href, '_blank')
    return
  }
  router.push({
    name: 'task-detail',
    params: { taskId },
  })
}
</script>
