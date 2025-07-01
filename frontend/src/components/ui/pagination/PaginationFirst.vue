<script setup lang="ts">
import { type HTMLAttributes, computed } from 'vue'
import { PaginationFirst, type PaginationFirstProps } from 'reka-ui'
import { Button } from '@/components/ui/button'
import { ChevronsLeft } from 'lucide-vue-next'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<PaginationFirstProps & { class?: HTMLAttributes['class'] }>(),
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
  <PaginationFirst v-bind="delegatedProps">
    <Button :class="cn('h-9 w-9 p-0', props.class)" variant="outline">
      <slot>
        <ChevronsLeft />
      </slot>
    </Button>
  </PaginationFirst>
</template>
