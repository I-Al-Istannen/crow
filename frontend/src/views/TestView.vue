<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row justify-between">
        <div class="flex flex-col">
          <CardTitle>Test all the things</CardTitle>
          <CardDescription>
            Browse all tests submitted by you or the course advisors
          </CardDescription>
        </div>
        <div>
          <CreateTestDialog>
            <Button variant="secondary">Create new test</Button>
          </CreateTestDialog>
        </div>
      </CardHeader>
      <CardContent v-auto-animate>
        <div v-if="isLoading">Loading tests...</div>
        <div v-if="isFetched && tests === undefined">Loading failed</div>
        <div v-if="isFetched && tests !== undefined">
          <Accordion type="multiple" v-model="expandedTests">
            <AccordionItem v-for="test in displayedTests" :key="test.id" :value="test.id">
              <AccordionTrigger>
                <span>
                  {{ test.name }}
                  <span class="text-sm text-muted-foreground ml-2">by {{ test.creator }}</span>
                </span>
              </AccordionTrigger>
              <AccordionContent>
                <TestDetail :test-id="test.id" />
              </AccordionContent>
            </AccordionItem>
          </Accordion>

          <Pagination
            class="mt-6"
            v-slot="{ page }"
            v-model:page="currentPage"
            :default-page="1"
            :items-per-page="itemsPerPage"
            :sibling-count="1"
            :total="tests.length"
            show-edges
            @update:page="expandedTests = []"
          >
            <PaginationList v-slot="{ items }" class="flex items-center gap-1">
              <PaginationFirst />
              <PaginationPrev />

              <template v-for="(item, index) in items">
                <PaginationListItem
                  v-if="item.type === 'page'"
                  :key="index"
                  :value="item.value"
                  as-child
                >
                  <Button
                    class="w-10 h-10 p-0"
                    :variant="item.value === page ? 'default' : 'outline'"
                  >
                    {{ item.value }}
                  </Button>
                </PaginationListItem>
                <PaginationEllipsis v-else :key="item.type" :index="index" />
              </template>

              <PaginationNext />
              <PaginationLast />
            </PaginationList>
          </Pagination>
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Pagination,
  PaginationEllipsis,
  PaginationFirst,
  PaginationLast,
  PaginationNext,
  PaginationPrev,
} from '@/components/ui/pagination'
import { PaginationList, PaginationListItem } from 'radix-vue'
import { computed, ref } from 'vue'
import { Button } from '@/components/ui/button'
import CreateTestDialog from '@/components/CreateTestDialog.vue'
import PageContainer from '@/components/PageContainer.vue'
import TestDetail from '@/components/TestDetail.vue'
import { queryTests } from '@/data/network.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const currentPage = ref<number>(1)
const itemsPerPage = ref<number>(3)
const expandedTests = ref<string[]>([])

const { data: tests, isFetched, isLoading } = queryTests()

const displayedTests = computed(() => {
  if (tests.value === undefined) {
    return undefined
  }
  const start = Math.max(0, currentPage.value - 1) * itemsPerPage.value
  const end = Math.min(tests.value.length, currentPage.value * itemsPerPage.value)

  return tests.value.slice(start, end)
})
</script>
