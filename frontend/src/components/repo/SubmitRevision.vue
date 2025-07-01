<template>
  <form novalidate @submit="onSubmit" class="space-y-4">
    <FormField v-slot="{ componentField }" name="revision" :validate-on-input="true">
      <FormItem v-auto-animate>
        <FormLabel class="text-sm font-medium">Revision (branch, commit, tag, etc.)</FormLabel>
        <FormControl>
          <Input type="text" placeholder="master" v-bind="componentField" />
        </FormControl>
        <FormDescription>The revision to add to the queue</FormDescription>
        <FormMessage />
      </FormItem>
    </FormField>
    <div class="flex items-center" v-auto-animate>
      <Button type="submit" :disabled="isPending">
        <LoaderCircle class="-ml-2 mr-2 animate-spin" v-show="isPending" />
        Submit
      </Button>
      <Button variant="link" v-if="resultingTaskId">
        <RouterLink
          class="gradient-primary"
          :to="{ name: 'task-detail', params: { taskId: resultingTaskId } }"
        >
          Take me to the task
        </RouterLink>
      </Button>
    </div>
  </form>
</template>

<script setup lang="ts">
import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import type { TaskId } from '@/types.ts'
import { mutateRequestRevision } from '@/data/network.ts'
import { ref } from 'vue'
import { toTypedSchema } from '@vee-validate/zod'
import { toast } from 'vue-sonner'
import { useForm } from 'vee-validate'
import { useQueryClient } from '@tanstack/vue-query'
import { useTimeoutFn } from '@vueuse/core'
import { vAutoAnimate } from '@formkit/auto-animate/vue'
import { z } from 'zod'

const resultingTaskId = ref<TaskId | null>(null)

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      revision: z.string().max(200, "That's a bit long for a revision, don't you think?"),
    }),
  ),
})

const { mutateAsync, isPending } = mutateRequestRevision(useQueryClient())
const { start } = useTimeoutFn(() => (resultingTaskId.value = null), 10_000)

const onSubmit = form.handleSubmit(async (values) => {
  const response = await mutateAsync(values.revision)
  if (response === null) {
    toast.error('Revision not found :/')
    return
  }
  resultingTaskId.value = response.taskId
  toast.success('Revision submitted :)')
  form.resetForm()
  start()
})
</script>
