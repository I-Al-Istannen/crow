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
        <div v-if="showSshWarning" class="flex items-center pl-2 text-xs text-orange-500">
          <div>
            <LucideTriangleAlert class="mr-2 size-4" />
          </div>
          <div>
            Using an SSH URL (instead of https) likely requires SSH keys to be set up and should
            only be needed for private repositories. Please contact us, if you want that :)
          </div>
        </div>
        <FormDescription>The URL to your repository</FormDescription>
        <FormMessage />
      </FormItem>
    </FormField>
    <Button type="submit" :disabled="mutationPending">
      <LoaderCircle class="-ml-2 mr-2 animate-spin" v-if="mutationPending" />
      Submit
    </Button>
    <div v-if="error !== null" class="whitespace-pre text-red-500">
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
import { LoaderCircle, LucideTriangleAlert } from 'lucide-vue-next'
import { computed, toRefs, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
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

const showSshWarning = computed(() => form.values.repoUrl?.includes('@'))

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
