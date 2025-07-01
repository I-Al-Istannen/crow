<template>
  <Dialog v-model:open="dialogOpen">
    <!-- prevent any outer elements (e.g. accordion) to trigger too -->
    <DialogTrigger @click.stop>
      <slot />
    </DialogTrigger>
    <DialogContent
      class="max-h-[80dvh] max-w-[98dvw] overflow-y-auto md:max-w-[90dvw] lg:max-w-[80dvw] xl:max-w-[70dvw]"
    >
      <DialogHeader>
        <DialogTitle v-if="editingExisting">Edit a test</DialogTitle>
        <DialogTitle v-else>Create a new test</DialogTitle>
        <DialogDescription>Share a test with the world and break some compilers</DialogDescription>
      </DialogHeader>
      <div class="overflow-hidden">
        <form novalidate @submit="onSubmit" class="grid grid-cols-1 gap-4 p-1 lg:grid-cols-2">
          <FormField v-slot="{ componentField }" name="id">
            <FormItem v-auto-animate class="flex-grow">
              <FormLabel class="text-sm font-medium">Name</FormLabel>
              <FormControl>
                <Input
                  :disabled="!!testToEdit"
                  type="text"
                  placeholder="Well hello compiler-friends"
                  v-bind="componentField"
                />
              </FormControl>
              <FormDescription>A unique identifier for your test</FormDescription>
              <FormMessage />
            </FormItem>
          </FormField>
          <FormField v-slot="{ componentField }" name="category">
            <FormItem v-auto-animate class="flex-grow">
              <FormLabel class="text-sm font-medium">Category</FormLabel>
              <Select v-bind="componentField">
                <FormControl>
                  <SelectTrigger>
                    <SelectValue placeholder="Select a category" />
                  </SelectTrigger>
                </FormControl>
                <SelectContent>
                  <SelectGroup v-if="categories">
                    <SelectItem v-for="category in categories" :key="category" :value="category">
                      {{ category }}
                    </SelectItem>
                  </SelectGroup>
                  <SelectGroup v-else>
                    <SelectItem value="loading" :disabled="true">Loading...</SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
              <FormDescription>The category your test belongs to</FormDescription>
            </FormItem>
          </FormField>
          <div>
            <span class="text-sm font-medium">Executing your compiler</span>
            <TestModifierList
              v-model:value="compilerModifiers"
              modifier-target="compiler"
              class="ml-2"
            />
          </div>
          <div>
            <span class="text-sm font-medium">Executing the compiled binary</span>
            <TestModifierList
              v-model:value="binaryModifiers"
              modifier-target="binary"
              class="ml-2"
            />
          </div>
          <FormField v-slot="{ value, handleChange }" type="checkbox" name="testTasting">
            <FormItem
              class="flex flex-row items-start gap-x-3 space-y-0 lg:col-span-2"
              v-auto-animate
            >
              <FormControl>
                <Checkbox :model-value="value" @update:model-value="handleChange" />
              </FormControl>
              <div class="space-y-1 leading-none" v-auto-animate>
                <FormLabel>Test Tasting</FormLabel>
                <FormDescription>
                  Only submit the test if it works against the reference compiler
                </FormDescription>
                <FormMessage />
              </div>
              <div class="ml-6 flex flex-grow justify-start self-center" v-auto-animate>
                <FinishedTestDetailDialog
                  v-if="clickedTest"
                  :test="clickedTest"
                  of-whom="reference"
                  v-model:dialog-open="failedTastingDialogOpen"
                  hide-test-content
                />
                <TooltipProvider v-if="failedTestTasting !== null">
                  <FinishedTestcaseSummaryIcon
                    :test="toFinishedTestSummary(failedTestTasting)"
                    @test-clicked="handleFailedTastingClick"
                    is-finished
                  />
                </TooltipProvider>
              </div>
            </FormItem>
          </FormField>
          <div class="col-start-1 flex items-center">
            <Button type="submit" :disabled="mutationPending">
              <LoaderCircle class="-ml-2 mr-2 animate-spin" v-show="editPending" />
              Submit
            </Button>
            <Button
              variant="destructive"
              :disabled="mutationPending"
              @click.stop.prevent="deleteTest"
              class="ml-2"
              v-if="testToEdit"
              v-auto-animate
            >
              <LoaderCircle class="-ml-2 mr-2 animate-spin" v-show="deletePending" />
              <span v-if="!inDeletionProcess">Delete</span>
              <span v-else>Confirm Deletion</span>
            </Button>
          </div>
        </form>
      </div>
    </DialogContent>
  </Dialog>
</template>

<script setup lang="ts">
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import {
  type FinishedTest,
  type Test,
  type TestId,
  type TestModifier,
  toFinishedTestSummary,
} from '@/types.ts'
import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { computed, ref, toRefs, watch } from 'vue'
import { mutateDeleteTest, mutateTest, queryTests } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import FinishedTestDetailDialog from '@/components/test-view/FinishedTestDetailDialog.vue'
import FinishedTestcaseSummaryIcon from '@/components/task-detail/FinishedTestcaseSummaryIcon.vue'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import TestModifierList from '@/components/test-edit/TestModifierList.vue'
import { TooltipProvider } from '@/components/ui/tooltip'
import { storeToRefs } from 'pinia'
import { toTypedSchema } from '@vee-validate/zod'
import { toast } from 'vue-sonner'
import { useForm } from 'vee-validate'
import { useQueryClient } from '@tanstack/vue-query'
import { useTimeoutFn } from '@vueuse/core'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'
import { z } from 'zod'

const inDeletionProcess = ref(false)
const editingExisting = ref(false)
const failedTestTasting = ref<FinishedTest | null>(null)
const clickedTest = ref<FinishedTest | null>(null)
const failedTastingDialogOpen = ref<boolean>(false)
const compilerModifiers = ref<(TestModifier & { key: number })[]>([])
const binaryModifiers = ref<(TestModifier & { key: number })[]>([])

const dialogOpen = defineModel<boolean>('open')
const props = defineProps<{
  testToEdit?: Test
}>()
const { testToEdit } = toRefs(props)

const { team } = storeToRefs(useUserStore())

const { mutateAsync: mutateEditTest, isPending: editPending } = mutateTest(useQueryClient())
const { data: testResponse } = queryTests()
const { mutateAsync: mutateDelTest, isPending: deletePending } = mutateDeleteTest(useQueryClient())
const mutationPending = computed(() => editPending.value || deletePending.value)
const tests = computed(() => testResponse.value?.tests)
const categories = computed(() => {
  if (!testResponse.value) {
    return undefined
  }
  return Array.from(Object.entries(testResponse.value.categories))
    .filter(([_name, meta]) => meta.startsAt <= new Date())
    .map(([name]) => name)
    .sort((a, b) => a.localeCompare(b))
})

const emit = defineEmits<{
  'test-deleted': [test_id: TestId]
}>()

const { start: startDeleteResetTimeout } = useTimeoutFn(
  () => {
    inDeletionProcess.value = false
  },
  2000,
  { immediate: false },
)

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      id: z
        .string()
        .min(3, 'Please give the test a descriptive id')
        .max(40, 'That id is a bit long, don’t you think?')
        .regex(/^[ a-zA-Z0-9_()-]+$/, 'Only alphanumeric characters, spaces and `_-()` are allowed')
        .refine((id) => !idTaken(id), 'This test id already exists'),
      category: z
        .string()
        .refine(
          (category) => categories.value?.includes(category),
          'Select a valid category: ' + (categories.value?.join(', ') ?? 'N/A'),
        ),
      testTasting: z.boolean(),
    }),
  ),
})

watch([dialogOpen, testToEdit], ([open, test]) => {
  editingExisting.value = false
  failedTestTasting.value = null

  if (open && test) {
    editingExisting.value = true
    form.resetForm({
      values: {
        id: test.id,
        category: test.category,
        testTasting: true,
      },
    })
    compilerModifiers.value = test.compilerModifiers.map((value, key) => ({
      ...value,
      key,
    }))
    binaryModifiers.value = test.binaryModifiers.map((value, key) => ({
      ...value,
      key,
    }))
  } else if (open) {
    form.resetForm({
      values: {
        id: undefined,
        category: undefined,
        testTasting: true,
      },
    })
    compilerModifiers.value = []
    binaryModifiers.value = []
  }
  inDeletionProcess.value = false
})

const idTaken = (id: TestId) => {
  if (tests.value === undefined || !team.value) {
    return false
  }
  if (testToEdit.value?.id === id) {
    return false
  }
  return !!tests.value.find((it) => it.id === id)
}

const onSubmit = form.handleSubmit(async (values) => {
  const res = await mutateEditTest({
    id: values.id,
    category: values.category,
    ignoreTestTasting: !values.testTasting,
    compilerModifiers: compilerModifiers.value,
    binaryModifiers: binaryModifiers.value,
  })

  if (res.type == 'TestAdded') {
    toast.success(testToEdit.value !== undefined ? 'Test updated :)' : 'Test created :)')
    dialogOpen.value = false
  } else {
    toast.error('The test failed on the reference compiler')
    failedTestTasting.value = {
      testId: values.id,
      output: res.output,
      category: values.category,
      provisionalForCategory: null,
    }
    form.setFieldError('testTasting', 'Failed on reference compiler. Details are on the right.')
  }
})

const deleteTest = async () => {
  if (!inDeletionProcess.value) {
    inDeletionProcess.value = true
    startDeleteResetTimeout()
    return
  }
  if (!testToEdit.value) {
    return
  }

  emit('test-deleted', testToEdit.value.id)
  await mutateDelTest(testToEdit.value.id)

  dialogOpen.value = false
}

function handleFailedTastingClick() {
  clickedTest.value = failedTestTasting.value
  failedTastingDialogOpen.value = true
}
</script>
