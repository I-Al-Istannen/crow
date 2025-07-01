<template>
  <Table class="w-fit">
    <TableHeader>
      <TableHead class="text-center">Place</TableHead>
      <TableHead class="text-center">Team</TableHead>
      <TableHead class="text-center">Tests</TableHead>
      <TableHead class="text-center">Time</TableHead>
      <TableHead class="text-center">Duration</TableHead>
      <TableHead class="text-center">Task</TableHead>
    </TableHeader>
    <TableBody>
      <TableRow v-for="([teamId, task], index) in sortedTeams" :key="teamId" class="cursor-pointer">
        <TableCell
          class="py-0 text-center"
          @click="goto(task.info.taskId, $event)"
          @click.middle="goto(task.info.taskId, $event)"
        >
          {{ index + 1 }}
        </TableCell>
        <TableCell v-if="isAdmin" class="py-0">
          <RouterLink :to="{ name: 'team-info', params: { teamId: task.info.teamId } }">
            <Button variant="link">
              {{ task.teamName }}
            </Button>
          </RouterLink>
        </TableCell>
        <TableCell
          v-else
          class="py-0"
          @click="goto(task.info.taskId, $event)"
          @click.middle="goto(task.info.taskId, $event)"
        >
          {{ task.teamName }}
        </TableCell>
        <TableCell
          class="py-0"
          @click="goto(task.info.taskId, $event)"
          @click.middle="goto(task.info.taskId, $event)"
        >
          <TaskQuickOverview class="text-sm" :task="task" />
        </TableCell>
        <TableCell
          class="py-0"
          @click="goto(task.info.taskId, $event)"
          @click.middle="goto(task.info.taskId, $event)"
        >
          {{ task.info.start.toLocaleString() }}
        </TableCell>
        <TableCell
          class="py-0"
          @click="goto(task.info.taskId, $event)"
          @click.middle="goto(task.info.taskId, $event)"
        >
          {{ formatDurationBetween(task.info.start, task.info.end) }}
        </TableCell>
        <TableCell class="py-0">
          <RouterLink :to="{ name: 'task-detail', params: { taskId: task.info.taskId } }">
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
import { formatDurationBetween } from '@/lib/utils.ts'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { useUserStore } from '@/stores/user.ts'

defineProps<{
  sortedTeams: [TeamId, ApiFinishedCompilerTaskSummary][]
}>()

const router = useRouter()
const { isAdmin } = storeToRefs(useUserStore())

function goto(taskId: TaskId, event: MouseEvent) {
  if (event.ctrlKey || event.button === 1) {
    window.open(router.resolve({ name: 'task-detail', params: { taskId } }).href, '_blank')
    return
  }
  void router.push({
    name: 'task-detail',
    params: { taskId },
  })
}
</script>
