<template>
  <div class="flex gap-2 w-full flex-wrap">
    <Input
      @update:model-value="table.setGlobalFilter($event)"
      placeholder="Filter..."
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
          <TableCell :colspan="columns.length" class="h-24 text-center"> No results.</TableCell>
        </TableRow>
      </template>
    </TableBody>
  </Table>
</template>

<script setup lang="ts">
import {
  type ColumnDef,
  FlexRender,
  type Table as TanstackTable,
  createColumnHelper,
  getCoreRowModel,
  getFilteredRowModel,
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
import { computed, h, toRefs } from 'vue'
import { Button } from '@/components/ui/button'
import DataTableColumnHeader from '@/components/ui/data-table/DataTableColumnHeader.vue'
import DataTableViewOptions from '@/components/ui/data-table/DataTableViewOptions.vue'
import { Input } from '@/components/ui/input'
import { statusColor } from '@/lib/utils.ts'

const props = defineProps<{
  tests: FinishedTest[]
  outdated: TestId[]
}>()

const { tests } = toRefs(props)

const emit = defineEmits<{
  testClicked: [test: FinishedTest]
}>()

const outdated = computed(() => new Set(props.outdated))

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
    cell: (val) => h('span', { class: 'text-muted-foreground' }, val.getValue() ? 'Outdated' : '-'),
  }),
  columnHelper.accessor((test) => test.provisionalForCategory, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Provisional',
      }),
    id: 'provisional',
    cell: (val) =>
      h(
        'span',
        {
          class: val.getValue() ? '' : 'text-muted-foreground',
        },
        `${val.getValue() || '-'}`,
      ),
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.display({
    header: 'Details',
    cell: (val) =>
      h(
        Button,
        { variant: 'link', onClick: () => emit('testClicked', val.row.original) },
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
  getFilteredRowModel: getFilteredRowModel(),
  getColumnCanGlobalFilter: (column) => column.columnDef.enableGlobalFilter !== false,
})
</script>
