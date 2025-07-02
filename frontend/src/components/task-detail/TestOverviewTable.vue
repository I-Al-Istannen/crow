<template>
  <div class="flex w-full flex-wrap gap-2">
    <Input
      @update:model-value="table.setGlobalFilter($event)"
      placeholder="Search..."
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
        <tr
          v-for="row in table.getRowModel().rows"
          :key="row.id"
          :data-state="row.getIsSelected() ? 'selected' : undefined"
          class="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted"
        >
          <td
            v-for="cell in row.getVisibleCells()"
            :key="cell.id"
            class="p-2 py-0 align-middle [&:has([role=checkbox])]:pr-0 [&>[role=checkbox]]:translate-y-0.5"
          >
            <span v-if="cell.column.id === 'testId'">{{ cell.getValue() }}</span>
            <span
              v-else-if="cell.column.id === 'status'"
              :class="statusColor(toExecutionStatus(cell.row.original.output), 'text')"
            >
              {{ cell.getValue() }}
            </span>
            <span v-else-if="cell.column.id === 'outdated'" class="text-muted-foreground">
              {{ cell.getValue() ? 'Outdated' : '-' }}
            </span>
            <span
              v-else-if="cell.column.id === 'provisional'"
              :class="{ 'text-muted-foreground': !cell.getValue() }"
            >
              {{ cell.getValue() ?? '-' }}
            </span>
            <span
              v-else-if="cell.column.id === 'category'"
              :class="{ 'text-muted-foreground': !cell.getValue() }"
            >
              {{ cell.getValue() ?? '-' }}
            </span>
            <span v-else-if="cell.column.id === 'details'">
              <button @click="emit('testClicked', cell.row.original)" class="py-1 hover:underline">
                Show details
              </button>
            </span>
          </td>
        </tr>
      </template>
      <template v-else>
        <TableRow>
          <TableCell :colspan="columns.length" class="h-24 text-center">No results.</TableCell>
        </TableRow>
      </template>
    </TableBody>
  </Table>
</template>

<script setup lang="ts">
import {
  type ColumnDef,
  FlexRender,
  type Row,
  type Table as TanstackTable,
  createColumnHelper,
  getCoreRowModel,
  getFacetedUniqueValues,
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
import { computed, h, onBeforeMount, onBeforeUpdate, onMounted, onUpdated, toRefs } from 'vue'
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

onBeforeMount(() => {
  console.time('Table render')
})
onMounted(() => {
  console.timeEnd('Table render')
})
onBeforeUpdate(() => {
  console.time('Table update')
})
onUpdated(() => {
  console.timeEnd('Table update')
})

const outdated = computed(() => new Set(props.outdated))

const columnHelper = createColumnHelper<FinishedTest>()
const isMultiSorting = computed(() => {
  return table.getState().sorting.length > 1
})

function boolFilterFn(row: Row<unknown>, columnId: string, filterValue: boolean | boolean[]) {
  const cellValue = row.getValue<boolean>(columnId)
  if (Array.isArray(filterValue)) {
    return filterValue.some((v) => cellValue === v)
  }
  return cellValue === filterValue
}

function arrIncludesHandleNullFilterFn(row: Row<unknown>, columnId: string, filterValue: unknown) {
  const cellValue = row.getValue<unknown>(columnId)
  if (Array.isArray(filterValue)) {
    return filterValue.some((v) => cellValue === v)
  }
  return cellValue === filterValue
}

const columns: ColumnDef<FinishedTest, never>[] = [
  columnHelper.accessor((test) => test.testId, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Name',
        hideValueFilter: true,
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
    filterFn: 'arrIncludesSome',
  }),
  columnHelper.accessor((test) => outdated.value.has(test.testId), {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Outdated',
      }),
    id: 'outdated',
    filterFn: boolFilterFn,
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.accessor((test) => test.provisionalForCategory, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Provisional',
      }),
    id: 'provisional',
    filterFn: arrIncludesHandleNullFilterFn,
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.accessor((test) => test.category, {
    header: (column) =>
      h(DataTableColumnHeader<FinishedTest>, {
        column: column.column,
        title: 'Category',
      }),
    id: 'category',
    filterFn: 'arrIncludesSome',
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
  columnHelper.display({
    header: 'Details',
    id: 'details',
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
  getFacetedUniqueValues: getFacetedUniqueValues(),
})
</script>
