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
      <div class="flex gap-2 w-full flex-wrap">
        <Input
          :model-value="table.getColumn('testId')?.getFilterValue() as string"
          @update:model-value="table.getColumn('testId')?.setFilterValue($event)"
          placeholder="Test name..."
          class="max-w-[30ch]"
        />
        <span class="flex-grow" />
        <DataTableViewOptions :table="table" />
      </div>
      <Table>
        <TableHeader>
          <TableRow v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
            <TableHead v-for="header in headerGroup.headers" :key="header.id">
              <FlexRender
                v-if="!header.placeholderId"
                :render="header.column.columnDef.header"
                :props="header.getContext()"
              />
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <template v-if="table.getRowModel().rows.length > 0">
            <TableRow
              v-for="row in table.getRowModel().rows"
              :key="row.id"
              :data-state="row.getIsSelected() ? 'selected' : undefined"
            >
              <TableCell v-for="cell in row.getVisibleCells()" :key="cell.id">
                <FlexRender :render="cell.column.columnDef.cell" :props="cell.getContext()" />
              </TableCell>
            </TableRow>
          </template>
          <template v-else>
            <TableRow>
              <TableCell :colspan="columns.length" class="h-24 text-center">
                No results.
              </TableCell>
            </TableRow>
          </template>
        </TableBody>
      </Table>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  type ColumnDef,
  FlexRender,
  type Table as TanstackTable,
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useVueTable,
} from '@tanstack/vue-table'
import { type FinishedTest, type TestId, toExecutionStatus } from '@/types.ts'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computed, h, ref, toRefs } from 'vue'
import { Button } from '@/components/ui/button'
import DataTableColumnHeader from '@/components/ui/data-table/DataTableColumnHeader.vue'
import DataTableViewOptions from '@/components/ui/data-table/DataTableViewOptions.vue'
import FinishedTestDetailDialog from '@/components/FinishedTestDetailDialog.vue'
import { Input } from '@/components/ui/input'
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

const columnHelper = createColumnHelper<FinishedTest>()
const isMultiSorting = computed(() => {
  return table.getState().sorting.length > 1
})

const columns: ColumnDef<FinishedTest, never>[] = [
  columnHelper.accessor((test) => test.testId, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Name',
      }),
    id: 'testId',
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.accessor((test) => test.output.type, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Status',
      }),
    id: 'status',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (cell) =>
      h(
        'span',
        { class: statusColor(toExecutionStatus(cell.row.original.output), 'text') },
        cell.getValue(),
      ),
  }),
  columnHelper.accessor((test) => outdated.value.has(test.testId), {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Outdated',
      }),
    id: 'outdated',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (val) =>
      val.getValue() ? h('span', { class: 'text-muted-foreground' }, 'Outdated') : '-',
  }),
  columnHelper.accessor((test) => test.provisionalForCategory, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Provisional',
      }),
    id: 'provisional',
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.display({
    header: 'Details',
    cell: (val) =>
      h(
        Button,
        { variant: 'outline', onClick: () => handleTestClick(val.row.original) },
        () => 'Show details',
      ),
  }),
]

const table: TanstackTable<FinishedTest> = useVueTable({
  get data() {
    return tests
  },
  get columns() {
    return columns
  },
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
})

const handleTestClick = (test: FinishedTest) => {
  clickedTest.value = test
  dialogOpen.value = true
}
</script>
