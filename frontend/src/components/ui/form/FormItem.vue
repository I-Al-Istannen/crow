<script lang="ts" setup>
import { type HTMLAttributes, provide } from 'vue'
import { FORM_ITEM_INJECTION_KEY } from './injectionKeys'
import { cn } from '@/lib/utils'
import { useId } from 'reka-ui'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()

const id = useId()
provide(FORM_ITEM_INJECTION_KEY, id)
</script>

<template>
  <div :class="cn('space-y-2', props.class)">
    <slot />
  </div>
</template>
