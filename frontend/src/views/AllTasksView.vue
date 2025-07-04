<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-col items-start justify-between sm:flex-row sm:items-center">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>All runs</CardTitle>
          <CardDescription>View all runs that ever ran against your code</CardDescription>
        </div>
        <div class="mr-2">
          <Command class="border">
            <CommandInput
              v-model="searchTerm"
              ref="searchInput"
              placeholder="Search for a run..."
            />
          </Command>
        </div>
      </CardHeader>
      <CardContent v-if="isFetched && data">
        <div class="space-y-1">
          <FinishedTaskOverview
            v-for="task in displayedTasks"
            :task="task"
            :key="task.info.taskId"
          />
        </div>

        <div v-if="displayedTasks.length === 0" class="mb-2 text-sm text-muted-foreground">
          <span v-if="searchTerm.length === 0">No tasks found to display</span>
          <span v-else>No task matches your query</span>
        </div>

        <PaginationControls
          class="mt-4"
          :data="data"
          @change="(_start, _end, slice) => (displayedTasks = slice)"
        />
      </CardContent>
      <CardContent v-if="isLoading">Loading recent tasks...</CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Command, CommandInput } from '@/components/ui/command'
import { computed, ref, watch } from 'vue'
import type { FinishedCompilerTaskSummary } from '@/types.ts'
import FinishedTaskOverview from '@/components/task-overview/FinishedTaskOverview.vue'
import PageContainer from '@/components/PageContainer.vue'
import PaginationControls from '@/components/PaginationControls.vue'
import { queryRecentTasks } from '@/data/network.ts'
import { useFilter } from 'reka-ui'
import { useMagicKeys } from '@vueuse/core'

const displayedTasks = ref<FinishedCompilerTaskSummary[]>([])
const searchInput = ref<InstanceType<typeof CommandInput>>()
const searchTerm = ref<string>('')

const { data: allData, isFetched, isLoading } = queryRecentTasks(0)

const { contains } = useFilter({ sensitivity: 'base' })

const data = computed(() => {
  if (!allData.value) {
    return undefined
  }

  return allData.value.filter(
    (task) =>
      contains(task.info.taskId, searchTerm.value) ||
      contains(task.info.revisionId, searchTerm.value) ||
      contains(task.info.commitMessage, searchTerm.value),
  )
})

const { CTRL_F } = useMagicKeys({
  passive: false,
  onEventFired: (event) => {
    if (event.key === 'f' && event.ctrlKey && !searchInput.value?.isFocused()) {
      event.preventDefault()
    }
  },
})

// Watch for CTRL+F key press to focus the search input
// https://github.com/vueuse/vueuse/issues/4822 for eslint ignore
// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
watch(CTRL_F!, (pressed) => {
  if (pressed) {
    searchInput.value?.focus()
  }
})
</script>
