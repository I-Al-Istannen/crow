<script setup lang="ts">
import { type HTMLAttributes, computed } from 'vue'
import { Separator } from 'reka-ui'
import type { SeparatorProps } from 'reka-ui'
import { cn } from '@/lib/utils'

const props = defineProps<SeparatorProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})
</script>

<template>
  <Separator v-bind="delegatedProps" :class="cn('-mx-1 h-px bg-border', props.class)">
    <slot />
  </Separator>
</template>
