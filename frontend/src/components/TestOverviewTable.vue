<template>
  <Card>
    <CardHeader>
      <CardTitle>Test results</CardTitle>
      <CardDescription>Information about individual tests</CardDescription>
    </CardHeader>
    <CardContent class="-mt-2" v-if="sortedTests.length === 0">
      No tests were run during this task.
    </CardContent>
    <CardContent class="flex flex-row gap-1 flex-wrap" v-else>
      <FinishedTestDetailDialog
        :test="clickedTest"
        :of-whom="ofWhom"
        v-model:dialog-open="dialogOpen"
      />
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>
              Status
              <LucideArrowDownAz class="inline" :size="16" />
              <LucideArrowUpAz class="inline" :size="16" />
            </TableHead>
            <TableHead>Outdated</TableHead>
            <TableHead>Provisional</TableHead>
            <TableHead>Details</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="test in sortedTests" :key="test.testId">
            <TableCell>{{ test.testId }}</TableCell>
            <TableCell :class="statusColor(toExecutionStatus(test.output), 'text')">
              {{ test.output.type }}
            </TableCell>
            <TableCell>
              <span v-if="outdated.has(test.testId)" class="text-muted-foreground">Outdated</span>
            </TableCell>
            <TableCell
              :class="{
                'text-muted-foreground': !outdated.has(test.testId),
              }"
            >
              {{ test.provisionalForCategory }}
            </TableCell>
            <TableCell>
              <Button variant="outline" @click="handleTestClick(test)">Show details</Button>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { type FinishedTest, type TestId, toExecutionStatus } from '@/types.ts'
import { LucideArrowDownAz, LucideArrowUpAz } from 'lucide-vue-next'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computed, ref, toRefs } from 'vue'
import { Button } from '@/components/ui/button'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import { statusColor } from '@/lib/utils.ts'

const clickedTest = ref<FinishedTest | undefined>(undefined)
const dialogOpen = ref<boolean>(false)

const props = defineProps<{
  tests: FinishedTest[]
  outdated: TestId[]
  ofWhom: 'reference' | 'yours'
}>()

const { ofWhom, tests } = toRefs(props)

const outdated = computed(() => new Set(props.outdated))

const sortedTests = computed(() =>
  tests.value.slice().sort((a, b) => a.testId.localeCompare(b.testId)),
)

const handleTestClick = (test: FinishedTest) => {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
