<script setup lang="ts">
import { type HTMLAttributes, computed } from 'vue'
import { PaginationLast, type PaginationLastProps } from 'reka-ui'
import { Button } from '@/components/ui/button'
import { ChevronsRight } from 'lucide-vue-next'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<PaginationLastProps & { class?: HTMLAttributes['class'] }>(),
  {
    asChild: true,
  },
)

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})
</script>

<template>
  <PaginationLast v-bind="delegatedProps">
    <Button :class="cn('w-9 h-9 p-0', props.class)" variant="outline">
      <slot>
        <ChevronsRight />
      </slot>
    </Button>
  </PaginationLast>
</template>
