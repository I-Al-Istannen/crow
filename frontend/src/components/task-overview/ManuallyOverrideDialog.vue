<template>
  <Popover v-model:open="isPopoverOpen">
    <PopoverTrigger
      :class="
        clsx(
          assignedCategories.length > 0 ? 'text-orange-500' : 'text-gray-800',
          assignedCategories.length > 0 ? 'opacity-100' : 'opacity-0',
          'transition-opacity',
          'group-hover:opacity-50',
          'hover:!opacity-100',
        )
      "
      class="flex items-stretch"
    >
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger class="flex items-center">
            <LucideUpload @click.prevent class="h-4 w-4" />
          </TooltipTrigger>
          <TooltipContent v-if="assignedCategories.length > 0">
            Submitting this task for {{ assignedCategories.join(' and ') }}. Click to change.
          </TooltipContent>
          <TooltipContent v-else>Configure manual overrides for task submission.</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </PopoverTrigger>
    <PopoverContent class="max-sm:w-[90dvw] w-[20em]">
      <DataLoadingExplanation
        :is-loading="isLoading"
        :failure-count="failureCount"
        :failure-reason="failureReason"
      />
      <div class="mb-2 text-sm">
        You can submit this solution for grading in specific labs. This will override the automatic
        selection.
      </div>
      <Select
        v-if="data"
        :model-value="assignedCategories"
        @update:model-value="overrideCategory"
        :disabled="isMutating"
        multiple
      >
        <SelectTrigger>
          <SelectValue placeholder="Labs" />
        </SelectTrigger>
        <SelectContent @closeAutoFocus="isPopoverOpen = false">
          <SelectGroup>
            <SelectItem v-for="category in availableCategories" :key="category" :value="category">
              {{ category }}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      <PopoverArrow class="fill-white stroke-gray-200" />
    </PopoverContent>
  </Popover>
</template>

<script setup lang="ts">
import { type AcceptableValue, PopoverArrow } from 'reka-ui'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { computed, ref, toRefs } from 'vue'
import {
  mutateSetFinalSubmittedTask,
  queryFinalSubmittedTasks,
  queryTests,
} from '@/data/network.ts'
import DataLoadingExplanation from '@/components/DataLoadingExplanation.vue'
import { LucideUpload } from 'lucide-vue-next'
import type { TaskId } from '@/types.ts'
import { clsx } from 'clsx'
import { toast } from 'vue-sonner'
import { useQueryClient } from '@tanstack/vue-query'

const isMutating = ref(false)
const isPopoverOpen = ref(false)

const props = defineProps<{
  taskId: TaskId
}>()
const { taskId } = toRefs(props)

const { data, isLoading, failureCount, failureReason } = queryTests()
const { data: gradedTasks } = queryFinalSubmittedTasks()
const { mutateAsync } = mutateSetFinalSubmittedTask(useQueryClient())

const availableCategories = computed(() => {
  if (!data.value) {
    return []
  }
  const validCategories = Array.from(Object.entries(data.value.categories))
    .filter(([_name, meta]) => meta.startsAt <= new Date() && new Date() <= meta.labsEndAt)
    .map(([name]) => name)

  const allCategories = new Set(validCategories)
  assignedCategories.value.forEach((it) => allCategories.add(it))

  return Array.from(allCategories.values()).sort((a, b) => a.localeCompare(b))
})

const assignedCategories = computed(() => {
  if (!gradedTasks.value) {
    return []
  }

  return Array.from(gradedTasks.value.entries())
    .filter(([, task]) => task.summary.info.taskId === props.taskId)
    .filter(([, task]) => task.type === 'ManuallyOverridden')
    .map(([category]) => category)
    .sort((a, b) => a.localeCompare(b))
})

const overrideCategory = async (newCategories: AcceptableValue) => {
  isMutating.value = true
  try {
    const categories = newCategories as string[]
    await mutateAsync([taskId.value, categories])

    if (categories.length === 0) {
      toast.success('Switched back to automatic submission')
    } else {
      toast.success(`Will submit this task for ${categories.join(', ')}`)
    }
  } finally {
    isMutating.value = false
  }
}
</script>
