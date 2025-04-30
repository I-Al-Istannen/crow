<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-col sm:flex-row justify-between items-start sm:items-center">
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
        />
        <div v-if="tests">
          <TooltipProvider>
            <Accordion type="multiple" v-model="expandedTests">
              <AccordionItem v-for="test in displayedTests" :key="test.id" :value="test.id">
                <AccordionTrigger>
                  <span class="flex items-center gap-1 max-sm:flex-wrap">
                    <Tooltip v-if="test.testTasteSuccess">
                      <TooltipTrigger as-child>
                        <LucideBadgeCheck :size="16" class="text-green-600" />
                      </TooltipTrigger>
                      <TooltipContent>This check passed on the reference compiler</TooltipContent>
                    </Tooltip>
                    <Tooltip v-if="test.testTasteSuccess === false">
                      <TooltipTrigger>
                        <LucideBadgeX :size="16" class="text-red-500" />
                      </TooltipTrigger>
                      <TooltipContent>
                        This check failed on the reference compiler. Open it for more details about
                        the failure.
                      </TooltipContent>
                    </Tooltip>
                    <Tooltip v-if="test.testTasteSuccess === null">
                      <TooltipTrigger>
                        <LucideBadgeAlert :size="16" class="text-gray-400" />
                      </TooltipTrigger>
                      <TooltipContent>
                        This test was never run against the reference compiler
                      </TooltipContent>
                    </Tooltip>
                    {{ test.id }}
                    <span class="text-sm text-muted-foreground">by {{ test.creatorName }}</span>
                    <Tooltip v-if="test.adminAuthored">
                      <TooltipTrigger as-child>
                        <LucideShieldCheck :size="16" />
                      </TooltipTrigger>
                      <TooltipContent>Created by an administrator</TooltipContent>
                    </Tooltip>
                    <Tooltip v-if="test.provisionalForCategory !== null">
                      <TooltipTrigger as-child>
                        <LucideEyeOff :size="16" class="text-orange-500" />
                      </TooltipTrigger>
                      <TooltipContent>
                        Created after test deadline. This test will only count in the next cycle.
                      </TooltipContent>
                    </Tooltip>
                  </span>
                  <span class="flex flex-grow justify-end mr-2 items-center gap-2">
                    <Button
                      v-if="canEdit(test)"
                      variant="ghost"
                      class="h-full p-2 -m-2"
                      @click.stop="openEditDialog(test)"
                      :disabled="testToEditLoading"
                    >
                      <LucidePencil :size="16" :class="{ 'animate-spin': testToEditLoading }" />
                    </Button>
                    <Badge variant="secondary">{{ test.category }}</Badge>
                  </span>
                </AccordionTrigger>
                <AccordionContent>
                  <TestDetail :test-id="test.id" />
                </AccordionContent>
              </AccordionItem>
            </Accordion>
          </TooltipProvider>

          <div v-if="tests.length === 0" class="text-muted-foreground text-sm mb-2">
            No tests yet. Create some!
          </div>

          <PaginationControls
            class="mt-4"
            :data="tests"
            @change="(_start, _end, slice) => (displayedTests = slice)"
          />
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
  LucideBadgeAlert,
  LucideBadgeCheck,
  LucideBadgeX,
  LucideEyeOff,
  LucidePencil,
  LucideShieldCheck,
} from 'lucide-vue-next'
import type { Test, TestId, TestSummary } from '@/types.ts'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { computed, ref, watch } from 'vue'
import { fetchTestDetail, queryTests } from '@/data/network.ts'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import PageContainer from '@/components/PageContainer.vue'
import PaginationControls from '@/components/PaginationControls.vue'
import SetTestDialog from '@/components/test-edit/SetTestDialog.vue'
import TestDetail from '@/components/test-view/TestDetail.vue'
import { storeToRefs } from 'pinia'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const expandedTests = ref<string[]>([])
const testSetDialogOpen = ref(false)
const testToEdit = ref<Test | undefined>(undefined)
const testToEditLoading = ref(false)
const displayedTests = ref<TestSummary[]>([])

const { isAdmin, team } = storeToRefs(useUserStore())
const { data: testResp, isLoading, failureCount, failureReason } = queryTests()

const tests = computed(() => sortTests(testResp.value?.tests))

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
</script>
