<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-col items-start justify-between sm:flex-row sm:items-center">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>Test all the things</CardTitle>
          <CardDescription>
            Browse all tests submitted by you or the course advisors
          </CardDescription>
        </div>
        <div>
          <SetTestDialog
            :test-to-edit="testToEdit"
            v-model:open="testSetDialogOpen"
            @test-deleted="handleTestDeleted"
          >
            <Button variant="secondary" :disabled="testToEditLoading">Create new test</Button>
          </SetTestDialog>
        </div>
      </CardHeader>
      <CardContent v-auto-animate>
        <DataLoadingExplanation
          :is-loading="isLoading"
          :failure-count="failureCount"
          :failure-reason="failureReason"
          class="mb-6"
        />
        <div v-if="allTests">
          <TooltipProvider v-if="allTests.length > 0">
            <Input
              type="text"
              placeholder="Search for test names, team names or category..."
              class="shadow-none"
              v-model="searchText"
            />
            <Accordion type="multiple" v-model="expandedTests">
              <TestListEntry v-for="test in displayedTests" :key="test.id" :test="test">
                <template #actions>
                  <Button
                    v-if="canEdit(test)"
                    variant="ghost"
                    class="-m-2 h-full p-2"
                    @click.stop="openEditDialog(test)"
                    :disabled="testToEditLoading"
                  >
                    <LucidePencil :size="16" :class="{ 'animate-spin': testToEditLoading }" />
                  </Button>
                </template>
              </TestListEntry>
            </Accordion>
          </TooltipProvider>

          <div
            v-if="displayedTests.length === 0 && allTests.length > 0"
            class="mx-2 mt-4 text-muted-foreground"
          >
            No test matches your search
          </div>

          <div v-if="allTests.length === 0" class="mb-2 text-sm text-muted-foreground">
            No tests yet. Create some!
          </div>

          <PaginationControls
            class="mt-4"
            :data="filteredTests"
            @change="(_start, _end, slice) => (displayedTests = slice)"
          />
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { Test, TestId, TestSummary } from '@/types.ts'
import { computed, ref, watch } from 'vue'
import { fetchTestDetail, queryTests } from '@/data/network.ts'
import { Accordion } from '@/components/ui/accordion'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import { Input } from '@/components/ui/input'
import { LucidePencil } from 'lucide-vue-next'
import PageContainer from '@/components/PageContainer.vue'
import PaginationControls from '@/components/PaginationControls.vue'
import SetTestDialog from '@/components/test-edit/SetTestDialog.vue'
import TestListEntry from '@/components/test-view/TestListEntry.vue'
import { TooltipProvider } from '@/components/ui/tooltip'
import { storeToRefs } from 'pinia'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const expandedTests = ref<string[]>([])
const testSetDialogOpen = ref(false)
const testToEdit = ref<Test | undefined>(undefined)
const testToEditLoading = ref(false)
const displayedTests = ref<TestSummary[]>([])
const searchText = ref('')

const { isAdmin, team } = storeToRefs(useUserStore())
const { data: testResp, isLoading, failureCount, failureReason } = queryTests()

const allTests = computed(() => sortTests(testResp.value?.tests))

const filteredTests = computed(() => {
  return (allTests.value ?? [])
    .slice()
    .filter((test) => doesTestMatchFilter(searchText.value, test))
})

// Reset the edited test so clicking on new test does not prefill with the last edited test
watch(testSetDialogOpen, (isOpen) => {
  if (!isOpen) {
    testToEdit.value = undefined
  }
})

async function openEditDialog(testSummary: TestSummary) {
  testToEditLoading.value = true
  try {
    const test = await fetchTestDetail(testSummary.id)
    if (test === null) {
      toast.error('Could not find test')
    } else {
      testToEdit.value = test
      testSetDialogOpen.value = true
    }
  } finally {
    testToEditLoading.value = false
  }
}

function canEdit(test: TestSummary): boolean {
  // Admins can edit all
  if (isAdmin.value) {
    return true
  }

  // I can only edit my own
  const isMine = test.creatorId === team.value?.id

  // If it is provisional, we can edit it
  if (test.provisionalForCategory === test.category) {
    return isMine
  }

  // If we have no category info, we fall back to "everything I can touch"
  if (testResp.value?.categories === undefined) {
    return isMine
  }

  // Check if the deadline has passed
  const category = Object.entries(testResp.value.categories).find(([id, _]) => id === test.category)
  if (category && category[1].testsEndAt < new Date()) {
    return false
  }

  return isMine
}

function sortTests(tests?: TestSummary[]): TestSummary[] | undefined {
  if (tests === undefined) {
    return undefined
  }
  return tests.slice().sort((a, b) => {
    if (b.category.localeCompare(a.category) !== 0) {
      return b.category.localeCompare(a.category)
    }
    return a.id.localeCompare(b.id)
  })
}

const handleTestDeleted = (testId: TestId) => {
  expandedTests.value = expandedTests.value.filter((id) => id !== testId)
}

function doesTestMatchFilter(filter: string, test: TestSummary) {
  const filterLower = filter.toLowerCase()
  return (
    test.id.toLowerCase().includes(filterLower) ||
    test.creatorName.toLowerCase().includes(filterLower) ||
    test.creatorId.toLowerCase().includes(filterLower) ||
    test.category.toLowerCase().includes(filterLower) ||
    (test.limitedToCategory && filterLower.includes('archived')) ||
    (test.provisionalForCategory !== null && filterLower.includes('provisional'))
  )
}
</script>
