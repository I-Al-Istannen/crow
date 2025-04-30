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
    <Button type="submit" :disabled="mutationPending">
      <LoaderCircle class="animate-spin mr-2 -ml-2" v-if="mutationPending" />
      Submit
    </Button>
    <div v-if="error !== null" class="text-red-500 whitespace-pre">
      {{ error }}
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
import { computed, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
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

const { mutateAsync, isPending: mutationPending, error } = mutateRepo(useQueryClient())

const props = defineProps<{
  repo?: Repo | null
}>()
const { repo } = toRefs(props)

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      repoUrl: z.string(),
    }),
  ),
  initialValues: {
    repoUrl: repo.value?.url,
  },
})

watch(repo, () => {
  form.resetForm({
    values: {
      repoUrl: repo.value?.url,
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
    },
  ])

  toast.success('Repository settings updated!')
})
</script>
