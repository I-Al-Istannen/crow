<template>
  <div class="flex gap-2 w-full flex-wrap">
    <Input
      @update:model-value="table.setGlobalFilter($event)"
      placeholder="Search user attribute..."
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
          <TableCell v-for="cell in row.getVisibleCells()" :key="cell.id" class="py-1">
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { computed, h, toRefs } from 'vue'
import { type AdminUserInfo } from '@/types.ts'
import DataTableColumnHeader from '@/components/ui/data-table/DataTableColumnHeader.vue'
import DataTableViewOptions from '@/components/ui/data-table/DataTableViewOptions.vue'
import { Input } from '@/components/ui/input'
import { RouterLink } from 'vue-router'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'

const props = defineProps<{
  users: AdminUserInfo[]
}>()

const { users } = toRefs(props)
const { user } = storeToRefs(useUserStore())

const columnHelper = createColumnHelper<AdminUserInfo>()
const isMultiSorting = computed(() => {
  return table.getState().sorting.length > 1
})

const columns: ColumnDef<AdminUserInfo, never>[] = [
  columnHelper.accessor((user) => user.displayName, {
    header: (column) =>
      h(DataTableColumnHeader<AdminUserInfo>, {
        column: column.column,
        title: 'Name',
      }),
    id: 'userName',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (cell) =>
      h(
        'span',
        { class: cell.row.original.id === user.value?.id ? 'gradient-primary' : '' },
        cell.getValue(),
      ),
  }),
  columnHelper.accessor((user) => user.id, {
    header: (column) =>
      h(DataTableColumnHeader<AdminUserInfo>, {
        column: column.column,
        title: 'Id',
      }),
    id: 'userId',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (cell) => h('span', {}, (cell.getValue() as string).substring(0, 20)),
  }),
  columnHelper.accessor((user) => user.role, {
    header: (column) =>
      h(DataTableColumnHeader<AdminUserInfo>, {
        column: column.column,
        title: 'Role',
      }),
    id: 'role',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (cell) =>
      h(
        'span',
        { class: cell.row.original.role === 'Admin' ? 'text-orange-500' : 'text-muted-foreground' },
        cell.getValue(),
      ),
  }),
  columnHelper.accessor((user) => user.team?.displayName, {
    header: (column) =>
      h(DataTableColumnHeader<AdminUserInfo>, {
        column: column.column,
        title: 'Team',
      }),
    id: 'team',
    meta: {
      isMultiSorting: isMultiSorting,
    },
    cell: (cell) => {
      const team = cell.row.original.team as AdminUserInfo['team']
      if (!team) {
        return h('span', { class: 'text-red-500' }, 'No team')
      }
      return h(
        RouterLink,
        {
          class: cell.getValue() ? 'hover:cursor-pointer hover:underline' : 'text-red-500',
          to: {
            name: 'team-info',
            params: { teamId: team.id },
          },
        },
        () => [team.displayName],
      )
    },
  }),
  columnHelper.accessor((user) => user.repoUrl, {
    header: (column) =>
      h(DataTableColumnHeader<AdminUserInfo>, {
        column: column.column,
        title: 'Repo',
      }),
    id: 'repo',
    cell: (cell) =>
      h(
        'a',
        {
          class: cell.getValue() ? 'hover:underline cursor-pointer' : 'text-muted-foreground',
          href: cell.getValue(),
        },
        cell.getValue() || 'No repo',
      ),
    meta: {
      isMultiSorting: isMultiSorting,
    },
  }),
]

const table: TanstackTable<AdminUserInfo> = useVueTable({
  get data() {
    return users
  },
  get columns() {
    return columns
  },
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  getFilteredRowModel: getFilteredRowModel(),
  getColumnCanGlobalFilter: (column) => column.columnDef.enableGlobalFilter !== false,
  initialState: {
    sorting: [
      {
        id: 'role',
        desc: false,
      },
      {
        id: 'userName',
        desc: false,
      },
    ],
  },
})
</script>
