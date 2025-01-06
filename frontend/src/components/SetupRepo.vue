<template>
  <form novalidate @submit="onSubmit" class="space-y-4">
    <FormField v-slot="{ componentField }" name="repoUrl" :validate-on-input="true">
      <FormItem v-auto-animate>
        <FormLabel class="text-sm font-medium">Repo URL</FormLabel>
        <FormControl>
          <Input
            type="text"
            placeholder="https://github.com/<user>/<repo>"
            v-bind="componentField"
          />
        </FormControl>
        <FormDescription>The URL to your repository</FormDescription>
        <FormMessage />
      </FormItem>
    </FormField>
    <FormField v-slot="{ value, handleChange }" name="autoFetch" :validate-on-input="true">
      <FormItem class="flex flex-row items-start gap-x-3 space-y-0">
        <FormControl>
          <Checkbox :checked="value" @update:checked="handleChange" />
        </FormControl>
        <div class="space-y-1 leading-none">
          <FormLabel>Automatic fetching</FormLabel>
          <FormDescription>
            Whether to automatically check for (and test) new commits
          </FormDescription>
          <FormMessage />
        </div>
      </FormItem>
    </FormField>
    <Button type="submit" :disabled="mutationPending">
      <LoaderCircle class="animate-spin mr-2 -ml-2" v-if="mutationPending" />
      Submit
    </Button>
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
import { computed, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import type { Repo } from '@/types.ts'
import { mutateRepo } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { toTypedSchema } from '@vee-validate/zod'
import { toast } from 'vue-sonner'
import { useForm } from 'vee-validate'
import { useQueryClient } from '@tanstack/vue-query'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'
import { z } from 'zod'

const { team } = storeToRefs(useUserStore())
const teamId = computed(() => team.value?.id)

const { mutateAsync, isPending: mutationPending } = mutateRepo(useQueryClient())

const props = defineProps<{
  repo?: Repo
}>()
const { repo } = toRefs(props)

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      repoUrl: z.string().url('invalid url'),
      autoFetch: z.boolean(),
    }),
  ),
  initialValues: {
    repoUrl: repo.value?.url,
    autoFetch: repo.value?.autoFetch,
  },
})

watch(repo, () => {
  form.resetForm({
    values: {
      repoUrl: repo.value?.url,
      autoFetch: repo.value?.autoFetch,
    },
  })
})

const onSubmit = form.handleSubmit(async (values) => {
  if (!teamId.value) {
    return
  }

  await mutateAsync([
    teamId.value,
    {
      repoUrl: values.repoUrl,
      autoFetch: values.autoFetch,
    },
  ])

  toast.success('Repository settings updated!')
})
</script>
