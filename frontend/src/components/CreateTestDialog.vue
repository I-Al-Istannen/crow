<template>
  <Dialog v-model:open="dialogOpen">
    <DialogTrigger>
      <slot />
    </DialogTrigger>
    <DialogContent class="max-w-[60dvw]">
      <DialogHeader>
        <DialogTitle>Create a new test</DialogTitle>
        <DialogDescription>Share a test with the world and break some compilers</DialogDescription>
      </DialogHeader>
      <div>
        <form novalidate @submit="onSubmit" class="space-y-4">
          <FormField v-slot="{ componentField }" name="name" :validate-on-input="true">
            <FormItem v-auto-animate>
              <FormLabel class="text-sm font-medium">Test display name</FormLabel>
              <FormControl>
                <Input type="text" placeholder="Cool test" v-bind="componentField" />
              </FormControl>
              <FormDescription>A descriptive name for your test</FormDescription>
              <FormMessage />
            </FormItem>
          </FormField>
          <FormField v-slot="{ componentField }" name="id" :validate-on-input="true">
            <FormItem v-auto-animate>
              <FormLabel class="text-sm font-medium">Test ID</FormLabel>
              <FormControl>
                <Input type="text" placeholder="cool-test" v-bind="componentField" />
              </FormControl>
              <FormDescription>A unique alpha-numeric identifier for your test</FormDescription>
              <FormMessage />
            </FormItem>
          </FormField>
          <FormField v-slot="{ componentField }" name="expectedOutput" :validate-on-input="true">
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
          <Button type="submit" :disabled="mutationPending">
            <LoaderCircle class="animate-spin mr-2 -ml-2" v-if="mutationPending" />
            Submit
          </Button>
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
import { mutateTest, queryTests } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import type { TestId } from '@/types.ts'
import { Textarea } from '@/components/ui/textarea'
import { ref } from 'vue'
import { storeToRefs } from 'pinia'
import { toTypedSchema } from '@vee-validate/zod'
import { toast } from 'vue-sonner'
import { useForm } from 'vee-validate'
import { useQueryClient } from '@tanstack/vue-query'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'
import { z } from 'zod'

const dialogOpen = ref(false)

const { team } = storeToRefs(useUserStore())

const { mutateAsync, isPending: mutationPending } = mutateTest(useQueryClient())
const { data: tests } = queryTests()

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      name: z
        .string()
        .min(3, 'Please give the test a descriptive name')
        .max(200, "That's a bit long, don't you think?"),
      id: z
        .string()
        .min(3, 'Please give the test a descriptive id')
        .max(40, 'That id is a bit long, don’t you think?')
        .regex(/[a-zA-Z0-9]+/, 'Only alphanumeric characters are allowed')
        .refine((id) => !idTaken(id), 'This test id already exists'),
      expectedOutput: z
        .string()
        .min(1, 'Some output would be nice')
        .max(15_000, 'Are you sure you need this?'),
    }),
  ),
})

const idTaken = (id: TestId) => {
  if (tests.value === undefined || !team.value) {
    return false
  }
  return !!tests.value.find((it) => it.id === id)
}

const onSubmit = form.handleSubmit(async (values) => {
  console.log('submitting', values)

  await mutateAsync({
    name: values.name,
    id: values.id,
    expectedOutput: values.expectedOutput,
  })

  toast.success('Test created :)')
  dialogOpen.value = false
})
</script>
