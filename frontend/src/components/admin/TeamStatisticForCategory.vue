<template>
  <div>
    <div class="text-large font-bold">{{ category }}</div>
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Team</TableHead>
          <TableHead>Task</TableHead>
          <TableHead>Points</TableHead>
          <TableHead>Total Tests</TableHead>
          <TableHead>Compile error</TableHead>
          <TableHead>Runtime error</TableHead>
          <TableHead>Bin exits</TableHead>
          <TableHead>Bin non-term</TableHead>
          <TableHead>Compiler-only test</TableHead>
          <TableHead>Unclassified</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="stat in stats" :key="stat.teamId">
          <TableCell>{{ stat.teamId }}</TableCell>
          <TableCell class="text-muted-foreground">
            <RouterLink
              v-if="stat.finalizedTask"
              :to="{ name: 'task-detail', params: { taskId: stat.finalizedTask.taskId } }"
              class="hover:underline"
            >
              {{ stat.finalizedTask.statistics.success.normal }}/{{
                stat.finalizedTask.statistics.total.normal
              }}
            </RouterLink>
            <span v-else>-</span>
          </TableCell>
          <TableCell class="text-muted-foreground">
            <span v-if="!stat.points">-</span>
            <span v-else-if="stat.points.points == 80" class="text-green-600">
              {{ stat.points?.points }}
            </span>
            <span v-else>{{ stat.points?.points }}</span>
          </TableCell>
          <TableCell>{{ stat.classification ? sumTests(stat.classification) : 0 }}</TableCell>
          <TableCell>{{ stat.classification?.compileError ?? 0 }}</TableCell>
          <TableCell>{{ stat.classification?.runtimeError ?? 0 }}</TableCell>
          <TableCell>{{ stat.classification?.exitCode ?? 0 }}</TableCell>
          <TableCell>{{ stat.classification?.nonTermination ?? 0 }}</TableCell>
          <TableCell>{{ stat.classification?.compilerSucceedNoExec ?? 0 }}</TableCell>
          <TableCell>{{ (stat.classification?.unclassified || []).join(',') }}</TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </div>
</template>

<script setup lang="ts">
import {
  type AdminFinalizedTask,
  type GradingPoints,
  type TeamId,
  type TeamStatistics,
  type TestClassification,
} from '@/types.ts'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computed } from 'vue'

interface StatForTeam {
  teamId: TeamId
  classification: TestClassification
  finalizedTask: AdminFinalizedTask | null
  points: GradingPoints | null
}

const props = defineProps<{
  statistics: TeamStatistics[]
  category: string
}>()

const stats = computed<StatForTeam[]>(() => {
  return props.statistics
    .map((stat) => {
      return {
        teamId: stat.team,
        classification: stat.testsPerCategory[props.category],
        finalizedTask: stat.finalizedTasksPerCategory[props.category]?.[0] ?? null,
        points: stat.finalizedTasksPerCategory[props.category]?.[1] ?? null,
      } as StatForTeam
    })
    .sort((a, b) => a.teamId.localeCompare(b.teamId))
})

function sumTests(classification: TestClassification): number {
  return Object.values(classification)
    .filter((it) => typeof it === 'number')
    .reduce((sum, count) => sum + count, 0)
}
</script>
