<template>
  <Pagination
    class="mt-6"
    v-slot="{ page }"
    v-model:page="currentPage"
    :default-page="1"
    :items-per-page="itemsPerPage"
    :sibling-count="1"
    :total="data.length"
    show-edges
    @update:page="expandedTests = []"
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
import { ref, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'

const currentPage = ref<number>(1)
const itemsPerPage = ref<number>(3)
const expandedTests = ref<string[]>([])

const props = defineProps<{
  data: T[]
}>()
const { data } = toRefs(props)

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
