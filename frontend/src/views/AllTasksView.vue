<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row justify-between items-center">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>All runs</CardTitle>
          <CardDescription>View all runs that ever ran against your code</CardDescription>
        </div>
        <div class="mr-2">
          <Command class="border" v-model:search-term="searchTerm">
            <CommandInput ref="searchInput" placeholder="Search for a run..." />
          </Command>
        </div>
      </CardHeader>
      <CardContent class="space-y-1" v-if="isFetched && data">
        <FinishedTaskOverview v-for="task in displayedTasks" :task="task" :key="task.info.taskId" />

        <div v-if="displayedTasks.length === 0" class="text-muted-foreground text-sm mb-2">
          <span v-if="searchTerm.length === 0">No tasks found to display</span>
          <span v-else>No task matches your query</span>
        </div>

        <PaginationControls
          :data="data"
          :items-per-page="15"
          v-show="displayedTasks.length !== data.length"
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
import FinishedTaskOverview from '@/components/FinishedTaskOverview.vue'
import PageContainer from '@/components/PageContainer.vue'
import PaginationControls from '@/components/PaginationControls.vue'
import { queryRecentTasks } from '@/data/network.ts'
import { useMagicKeys } from '@vueuse/core'

const displayedTasks = ref<FinishedCompilerTaskSummary[]>([])
const searchInput = ref<InstanceType<typeof CommandInput>>()
const searchTerm = ref<string>('')

const { data: allData, isFetched, isLoading } = queryRecentTasks(0)

const data = computed(() => {
  if (!allData.value) {
    return undefined
  }

  return allData.value.filter(
    (task) =>
      task.info.taskId.includes(searchTerm.value) ||
      task.info.revisionId.includes(searchTerm.value),
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

watch(CTRL_F, (pressed) => {
  if (pressed) {
    searchInput.value?.focus()
  }
})
</script>
