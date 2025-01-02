<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Repository settings</CardTitle>
        <CardDescription>Change your repository URL and crows behaviour</CardDescription>
      </CardHeader>
      <CardContent>
        <form @submit="onSubmit" class="space-y-4">
          <FormField v-slot="{ componentField }" name="repoUrl">
            <FormItem>
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
          <FormField v-slot="{ value, handleChange }" name="autoFetch">
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
          <Button type="submit">Submit</Button>
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
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import PageContainer from '@/components/PageContainer.vue'
import { PatchRepoSchema } from '@/types.ts'
import { toTypedSchema } from '@vee-validate/zod'
import { useForm } from 'vee-validate'

const form = useForm({
  validationSchema: toTypedSchema(PatchRepoSchema),
})

const onSubmit = form.handleSubmit((values) => {
  console.log(values)
})
</script>
