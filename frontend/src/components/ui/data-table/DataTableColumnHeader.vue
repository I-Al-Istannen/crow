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
            class="-ml-1 text-xs -translate-y-1 font-mono"
            :class="{ 'opacity-0': !isMultiSorting || column.getSortIndex() < 0 }"
          >
            {{ column.getSortIndex() + 1 }}
          </span>
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
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { LucideArrowDownZA, LucideArrowUpAZ, LucideArrowUpDown } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { computed } from 'vue'

interface DataTableColumnHeaderProps {
  column: Column<T>
  title: string
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

function toggleSorting(dir: SortDirection) {
  if (dir == props.column.getIsSorted()) {
    props.column.clearSorting()
    return
  }
  props.column.toggleSorting(dir === 'desc', true)
}
</script>
