<script setup lang="ts">
import { type HTMLAttributes, computed } from 'vue'
import { PaginationNext, type PaginationNextProps } from 'reka-ui'
import { Button } from '@/components/ui/button'
import { ChevronRight } from 'lucide-vue-next'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<PaginationNextProps & { class?: HTMLAttributes['class'] }>(),
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
  <PaginationNext v-bind="delegatedProps">
    <Button :class="cn('w-9 h-9 p-0', props.class)" variant="outline">
      <slot>
        <ChevronRight />
      </slot>
    </Button>
  </PaginationNext>
</template>
