<template>
  <Card>
    <CardHeader>
      <CardTitle>Rerun submissions</CardTitle>
      <CardDescription>Rerun all submissions for a category for grading</CardDescription>
    </CardHeader>
    <CardContent>
      <DataLoadingExplanation
        :isLoading="testsLoading"
        :failureCount="testsFailureCount"
        :failureReason="testsFailureReason"
      />
      <form
        novalidate
        @submit="onSubmit"
        class="grid grid-cols-1 gap-4 p-1 lg:grid-cols-2"
        v-if="categories"
      >
        <FormField v-slot="{ componentField }" name="category">
          <FormItem class="flex-grow">
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
            <FormDescription>The category to trigger a rerun for</FormDescription>
          </FormItem>
        </FormField>
        <div class="col-start-1 flex items-center">
          <Button type="submit" :disabled="rerunPending">
            <LoaderCircle class="-ml-2 mr-2 animate-spin" v-show="rerunPending" />
            Rerun submissions
          </Button>
        </div>
      </form>

      <div class="mt-4 text-destructive" v-if="rerunError">
        Rerunning submissions failed
        <br />
        {{ rerunError }}
      </div>
      <ul class="mx-4 mt-4 list-disc text-sm" v-if="rerunResult">
        <li v-for="error in rerunResult.errors" :key="error">
          <pre class="text-destructive">{{ error }}</pre>
        </li>
        <li v-for="[team, task] in rerunResult.submitted" :key="team">
          <span class="text-muted-foreground">Submitted task for</span> {{ team }},
          <span class="text-muted-foreground">find it</span>
          <RouterLink
            :to="{ name: 'task-detail', params: { taskId: task } }"
            class="cursor:pointer hover:underline"
          >
            here
          </RouterLink>
        </li>
      </ul>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { FormControl, FormDescription, FormField, FormItem, FormLabel } from '@/components/ui/form'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { mutateRerunForGrading, queryTests } from '@/data/network.ts'
import { Button } from '@/components/ui/button'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import { LoaderCircle } from 'lucide-vue-next'
import { computed } from 'vue'
import { toTypedSchema } from '@vee-validate/zod'
import { useForm } from 'vee-validate'
import { z } from 'zod'

const {
  data: tests,
  isLoading: testsLoading,
  failureCount: testsFailureCount,
  failureReason: testsFailureReason,
} = queryTests()

const categories = computed(() => {
  if (!tests.value) {
    return undefined
  }

  return Array.from(Object.entries(tests.value.categories))
    .filter(([_name, meta]) => meta.startsAt <= new Date() && meta.labsEndAt <= new Date())
    .map(([name]) => name)
    .sort((a, b) => a.localeCompare(b))
})

const {
  mutateAsync: rerunSubmissions,
  isPending: rerunPending,
  error: rerunError,
  data: rerunResult,
} = mutateRerunForGrading()

const form = useForm({
  validationSchema: toTypedSchema(
    z.object({
      category: z
        .string()
        .refine(
          (category) => categories.value?.includes(category),
          'Select a valid category: ' + (categories.value?.join(', ') ?? 'N/A'),
        ),
    }),
  ),
})

const onSubmit = form.handleSubmit(async (values) => {
  await rerunSubmissions(values.category)
})
</script>
