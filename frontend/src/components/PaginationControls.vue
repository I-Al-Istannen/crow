<template>
  <Pagination
    v-slot="{ page }"
    v-model:page="currentPage"
    :default-page="1"
    :items-per-page="itemsPerPage"
    :sibling-count="1"
    :total="data.length"
    show-edges
    @update:page="expandedTests = []"
    v-show="showAlways || data.length > allowedItemsPerPage[0][0]"
  >
    <PaginationList v-slot="{ items }" class="flex items-center gap-1">
      <PaginationFirst />
      <PaginationPrev />

      <template v-for="(item, index) in items">
        <PaginationListItem v-if="item.type === 'page'" :key="index" :value="item.value" as-child>
          <Button class="w-10 h-10 p-0" :variant="item.value === page ? 'default' : 'outline'">
            {{ item.value }}
          </Button>
        </PaginationListItem>
        <PaginationEllipsis v-else :key="item.type" :index="index" />
      </template>

      <PaginationNext />
      <PaginationLast />

      <span class="flex-grow"></span>
      <div>
        <Select
          :model-value="itemsPerPage + ''"
          @update:model-value="itemsPerPage = parseInt($event)"
        >
          <SelectTrigger>
            <SelectValue placeholder="Hello" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectLabel>Items per Page</SelectLabel>
              <SelectItem
                v-for="[value, label] in allowedItemsPerPage"
                :key="value"
                :value="value + ''"
              >
                {{ label }}
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
    </PaginationList>
  </Pagination>
</template>

<script setup lang="ts" generic="T">
import {
  Pagination,
  PaginationEllipsis,
  PaginationFirst,
  PaginationLast,
  PaginationNext,
  PaginationPrev,
} from '@/components/ui/pagination'
import { PaginationList, PaginationListItem } from 'radix-vue'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { computed, ref, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'

const SHOW_ALL_ITEMS = 10000000

const currentPage = ref<number>(1)
const expandedTests = ref<string[]>([])

const itemsPerPage = defineModel<number>('itemsPerPage', { default: 10 })

const props = defineProps<{
  data: T[]
  showAlways?: boolean
}>()
const { data, showAlways } = toRefs(props)

const allowedItemsPerPage = computed(() => {
  const choices: [number, string][] = [
    [5, '5'],
    [10, '10'],
    [25, '25'],
    [50, '50'],
    [100, '100'],
    [250, '250'],
    [SHOW_ALL_ITEMS, 'all'],
  ]
  if (choices.findIndex((it) => it[0] == itemsPerPage.value) < 0) {
    choices.push([itemsPerPage.value, itemsPerPage.value.toString()])
  }
  choices.sort(([a, _], [b, __]) => a - b)

  return choices
})

const emit = defineEmits<{
  change: [start: number, end: number, slice: T[]]
}>()

watch(
  [currentPage, itemsPerPage, data],
  ([currentPage, itemsPerPage, data]) => {
    const start = Math.max(0, currentPage - 1) * itemsPerPage
    const end = Math.min(data.length, currentPage * itemsPerPage)
    const slice = data.slice(start, end)

    emit('change', start, end, slice)
  },
  { immediate: true },
)
</script>
