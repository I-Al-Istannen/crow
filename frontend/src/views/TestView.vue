<template>
  <PageContainer>
    <Card>
      <CardHeader class="flex flex-row justify-between">
        <div class="flex flex-col gap-y-1.5">
          <CardTitle>Test all the things</CardTitle>
          <CardDescription>
            Browse all tests submitted by you or the course advisors
          </CardDescription>
        </div>
        <div>
          <SetTestDialog :test-to-edit="testToEdit" v-model:open="testSetDialogOpen">
            <Button variant="secondary" :disabled="testToEditLoading">Create new test</Button>
          </SetTestDialog>
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
                  <span class="text-sm text-muted-foreground ml-2">by {{ test.creatorName }}</span>
                </span>
                <span class="flex flex-grow justify-end mr-2" v-if="canEdit(test)">
                  <Button
                    variant="ghost"
                    class="h-full p-2 -m-2"
                    @click.stop="openEditDialog(test)"
                    :disabled="testToEditLoading"
                  >
                    <LucidePencil :size="16" :class="{ 'animate-spin': testToEditLoading }" />
                  </Button>
                </span>
              </AccordionTrigger>
              <AccordionContent>
                <TestDetail :test-id="test.id" />
              </AccordionContent>
            </AccordionItem>
          </Accordion>

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
import type { Test, TestSummary } from '@/types.ts'
import { fetchTestDetail, queryTests } from '@/data/network.ts'
import { ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { LucidePencil } from 'lucide-vue-next'
import PageContainer from '@/components/PageContainer.vue'
import PaginationControls from '@/components/PaginationControls.vue'
import SetTestDialog from '@/components/SetTestDialog.vue'
import TestDetail from '@/components/TestDetail.vue'
import { storeToRefs } from 'pinia'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const expandedTests = ref<string[]>([])
const testSetDialogOpen = ref(false)
const testToEdit = ref<Test | undefined>(undefined)
const testToEditLoading = ref(false)
const displayedTests = ref<TestSummary[]>([])

const { team } = storeToRefs(useUserStore())
const { data: tests, isFetched, isLoading } = queryTests()

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
  return test.creatorId === team.value?.id
}
</script>
