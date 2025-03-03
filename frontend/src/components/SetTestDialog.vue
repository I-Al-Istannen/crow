<template>
  <Dialog v-model:open="dialogOpen">
    <!-- prevent any outer elements (e.g. accordion) to trigger too -->
    <DialogTrigger @click.stop>
      <slot />
    </DialogTrigger>
    <DialogContent class="max-w-[60dvw] max-h-[80dvh] overflow-y-auto">
      <DialogHeader>
        <DialogTitle v-if="editingExisting">Edit a test</DialogTitle>
        <DialogTitle v-else>Create a new test</DialogTitle>
        <DialogDescription>Share a test with the world and break some compilers</DialogDescription>
      </DialogHeader>
      <div>
        <form novalidate @submit="onSubmit" class="space-y-4">
          <div class="flex gap-2 items-start flex-wrap">
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
          </div>
          <FormField v-slot="{ componentField }" name="input">
            <FormItem v-auto-animate>
              <FormLabel class="text-sm font-medium">Input</FormLabel>
              <FormControl>
                <Textarea
                  v-bind="componentField"
                  class="font-mono whitespace-pre"
                  placeholder="Input..."
                />
              </FormControl>
              <FormDescription>
                The contents of the input file your compiler receives
              </FormDescription>
              <FormMessage />
            </FormItem>
          </FormField>
          <FormField v-slot="{ componentField }" name="expectedOutput">
            <FormItem v-auto-animate>
              <FormLabel class="text-sm font-medium">Expected output</FormLabel>
              <FormControl>
                <Textarea
                  v-bind="componentField"
                  class="font-mono whitespace-pre"
                  placeholder="Output..."
                />
              </FormControl>
              <FormDescription>The output your test should produce</FormDescription>
              <FormMessage />
            </FormItem>
          </FormField>
          <div class="flex items-center">
            <Button type="submit" :disabled="mutationPending">
              <LoaderCircle class="animate-spin mr-2 -ml-2" v-show="editPending" />
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
              <LoaderCircle class="animate-spin mr-2 -ml-2" v-show="deletePending" />
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
import type { Test, TestId } from '@/types.ts'
import { computed, ref, toRefs, watch } from 'vue'
import { mutateDeleteTest, mutateTest, queryTests } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import { Textarea } from '@/components/ui/textarea'
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
const categories = computed(() => testResponse.value?.categories)

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
        .regex(/^[ a-zA-Z0-9_-]+$/, 'Only alphanumeric characters and spaces are allowed')
        .refine((id) => !idTaken(id), 'This test id already exists'),
      category: z
        .string()
        .refine(
          (category) => categories.value?.includes(category),
          'Select a valid category: ' + categories.value?.join(', '),
        ),
      input: z
        .string()
        .min(1, 'The input file to compile')
        .max(150_000, 'Are you sure you need this much?'),
      expectedOutput: z
        .string()
        .min(1, 'Some output would be nice')
        .max(15_000, 'Are you sure you need this?'),
    }),
  ),
})

watch([dialogOpen, testToEdit], ([open, test]) => {
  editingExisting.value = false

  if (open && test) {
    editingExisting.value = true
    form.resetForm({
      values: {
        input: test.input,
        id: test.id,
        expectedOutput: test.expectedOutput,
        category: test.category,
      },
    })
  } else if (open) {
    form.resetForm({
      values: {
        input: undefined,
        id: undefined,
        expectedOutput: undefined,
        category: undefined,
      },
    })
  }
  inDeletionProcess.value = false
})

const idTaken = (id: TestId) => {
  if (tests.value === undefined || !team.value) {
    return false
  }
  if (testToEdit?.value?.id === id) {
    return false
  }
  return !!tests.value.find((it) => it.id === id)
}

const onSubmit = form.handleSubmit(async (values) => {
  await mutateEditTest({
    input: values.input,
    id: values.id,
    expectedOutput: values.expectedOutput,
    category: values.category,
  })

  toast.success(testToEdit?.value !== undefined ? 'Test updated :)' : 'Test created :)')
  dialogOpen.value = false
})

const deleteTest = async () => {
  if (!inDeletionProcess.value) {
    inDeletionProcess.value = true
    startDeleteResetTimeout()
    return
  }
  if (!testToEdit?.value) {
    return
  }

  await mutateDelTest(testToEdit?.value?.id)

  dialogOpen.value = false
}
</script>
