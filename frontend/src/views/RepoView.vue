<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Repository settings</CardTitle>
        <CardDescription>Change your repository URL and crows behaviour</CardDescription>
      </CardHeader>
      <CardContent v-if="isLoading">Requesting repo information...</CardContent>
      <CardContent v-show="isFetched">
        <form novalidate @submit="onSubmit" class="space-y-4" :inert="!isFetched">
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
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { computed, watch } from 'vue'
import { mutateRepo, queryRepo } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { LoaderCircle } from 'lucide-vue-next'
import PageContainer from '@/components/PageContainer.vue'
import { PatchRepoSchema } from '@/types.ts'
import { storeToRefs } from 'pinia'
import { toTypedSchema } from '@vee-validate/zod'
import { toast } from 'vue-sonner'
import { useForm } from 'vee-validate'
import { useQueryClient } from '@tanstack/vue-query'
import { useUserStore } from '@/stores/user.ts'
import { vAutoAnimate } from '@formkit/auto-animate/vue'

const { team } = storeToRefs(useUserStore())
const teamId = computed(() => team.value?.id)

const { data: repo, isFetched, isLoading } = queryRepo(teamId)
const { mutateAsync, isPending: mutationPending } = mutateRepo(useQueryClient())

const form = useForm({
  validationSchema: toTypedSchema(PatchRepoSchema),
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
