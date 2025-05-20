<template>
  <AccordionItem :value="test.id">
    <AccordionTrigger>
      <span class="flex items-center gap-1 max-sm:flex-wrap">
        <Tooltip v-if="test.testTasteSuccess">
          <TooltipTrigger as-child>
            <LucideBadgeCheck :size="16" class="text-green-600" />
          </TooltipTrigger>
          <TooltipContent>This check passed on the reference compiler</TooltipContent>
        </Tooltip>
        <Tooltip v-if="test.testTasteSuccess === false">
          <TooltipTrigger>
            <LucideBadgeX :size="16" class="text-red-500" />
          </TooltipTrigger>
          <TooltipContent>
            This check failed on the reference compiler. Open it for more details about the failure.
          </TooltipContent>
        </Tooltip>
        <Tooltip v-if="test.testTasteSuccess === null">
          <TooltipTrigger>
            <LucideBadgeAlert :size="16" class="text-gray-400" />
          </TooltipTrigger>
          <TooltipContent> This test was never run against the reference compiler </TooltipContent>
        </Tooltip>
        {{ test.id }}
        <span class="text-sm text-muted-foreground">by {{ test.creatorName }}</span>
        <Tooltip v-if="test.adminAuthored">
          <TooltipTrigger as-child>
            <LucideShieldCheck :size="16" />
          </TooltipTrigger>
          <TooltipContent>Created by an administrator</TooltipContent>
        </Tooltip>
        <Tooltip v-if="test.provisionalForCategory !== null">
          <TooltipTrigger as-child>
            <LucideEyeOff :size="16" class="text-orange-500" />
          </TooltipTrigger>
          <TooltipContent>
            Created after test deadline. This test will only count in the next cycle.
          </TooltipContent>
        </Tooltip>
        <Tooltip v-if="test.limitedToCategory">
          <TooltipTrigger as-child>
            <LucideFileArchive :size="16" class="text-orange-500" />
          </TooltipTrigger>
          <TooltipContent>
            This test is only relevant for the category it was submitted for. It will no longer be
            executed for newer categories.
          </TooltipContent>
        </Tooltip>
      </span>
      <span class="flex flex-grow justify-end mr-2 items-center gap-2">
        <slot name="actions" />
        <Badge variant="secondary">{{ test.category }}</Badge>
      </span>
    </AccordionTrigger>
    <AccordionContent>
      <TestDetail :test-id="test.id" />
    </AccordionContent>
  </AccordionItem>
</template>

<script setup lang="ts">
import { AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion'
import {
  LucideBadgeAlert,
  LucideBadgeCheck,
  LucideBadgeX,
  LucideEyeOff,
  LucideFileArchive,
  LucideShieldCheck,
} from 'lucide-vue-next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { Badge } from '@/components/ui/badge'
import TestDetail from '@/components/test-view/TestDetail.vue'
import type { TestSummary } from '@/types.ts'

defineProps<{
  test: TestSummary
}>()
</script>
