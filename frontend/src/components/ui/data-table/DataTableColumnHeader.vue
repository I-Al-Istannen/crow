<template>
  <div v-if="column.getCanSort()" :class="cn('flex items-center space-x-2', $attrs.class ?? '')">
    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button variant="ghost" size="xs" class="-ml-2 h-8 data-[state=open]:bg-accent">
          <span>{{ title }}</span>
          <LucideArrowDownZA v-if="column.getIsSorted() === 'desc'" class="w-4 h-4" />
          <LucideArrowUpAZ v-else-if="column.getIsSorted() === 'asc'" class="w-4 h-4" />
          <LucideArrowUpDown v-else class="w-4 h-4" />
          <span
            class="-ml-[0.4rem] text-xs -translate-y-1 font-mono"
            :class="{ 'opacity-0': !isMultiSorting || column.getSortIndex() < 0 }"
          >
            {{ column.getSortIndex() + 1 }}
          </span>
          <LucideFilter
            v-if="column.getFilterValue() !== undefined"
            class="w-4 h-4"
            :class="[!isMultiSorting || column.getSortIndex() < 0 ? '-ml-3' : '-ml-1']"
          />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        <DropdownMenuCheckboxItem
          :model-value="column.getIsSorted() === 'asc'"
          @click="toggleSorting('asc')"
          class="cursor-pointer"
        >
          <LucideArrowUpAZ class="mr-2 h-3.5 w-3.5 text-muted-foreground/70" />
          Asc
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          :model-value="column.getIsSorted() === 'desc'"
          @click="toggleSorting('desc')"
          class="cursor-pointer"
        >
          <LucideArrowDownZA class="mr-2 h-3.5 w-3.5 text-muted-foreground/70" />
          Desc
        </DropdownMenuCheckboxItem>
        <DropdownMenuSeparator v-if="uniqueValues.length > 1" />
        <DropdownMenuGroup v-if="uniqueValues.length > 1">
          <DropdownMenuLabel class="text-xs">Filter</DropdownMenuLabel>
          <DropdownMenuCheckboxItem
            v-for="value in uniqueValues"
            :key="value"
            :model-value="isFiltered(value)"
            @update:model-value="toggleFilterValue(value)"
            @select="$event.preventDefault()"
          >
            <span v-if="value === true">Yes</span>
            <span v-else-if="value === false">No</span>
            <span v-else-if="value === null || value === undefined" class="text-muted-foreground">
              None
            </span>
            <span v-else>{{ value }}</span>
          </DropdownMenuCheckboxItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>

  <div v-else :class="$attrs.class">
    {{ title }}
  </div>
</template>

<script setup lang="ts" generic="T">
import type { Column, SortDirection } from '@tanstack/vue-table'
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  LucideArrowDownZA,
  LucideArrowUpAZ,
  LucideArrowUpDown,
  LucideFilter,
} from 'lucide-vue-next'
import { type Ref, computed } from 'vue'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface DataTableColumnHeaderProps {
  column: Column<T>
  title: string
  potentialValues?: Ref<string[]>
}

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<DataTableColumnHeaderProps>()

const isMultiSorting = computed(() => {
  const meta = props.column.columnDef.meta
  if (!meta) {
    return false
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return (meta as any).isMultiSorting.value
})

const uniqueValues = computed(() => {
  const values = props.column.getFacetedUniqueValues().keys()
  return Array.from(values).sort((a, b) => {
    if (a === null || a === undefined) return -1
    if (b === null || b === undefined) return 1
    return a.localeCompare(b)
  })
})

function toggleSorting(dir: SortDirection) {
  if (dir == props.column.getIsSorted()) {
    props.column.clearSorting()
    return
  }
  props.column.toggleSorting(dir === 'desc', true)
}

function isFiltered(value: string): boolean {
  const filterValue = (props.column.getFilterValue() as string[]) || []
  return filterValue.includes(value)
}

function toggleFilterValue(value: string) {
  const currentFilterValue = (props.column.getFilterValue() as string[]) || []

  const newFilterValue = currentFilterValue.includes(value)
    ? currentFilterValue.filter((v) => v !== value)
    : [...currentFilterValue, value]

  props.column.setFilterValue(newFilterValue.length > 0 ? newFilterValue : undefined)
}
</script>
