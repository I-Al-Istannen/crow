<template>
  <div
    v-if="readonly"
    v-bind="$attrs"
    class="self-stretch shrink-0 rounded-md h-7 px-2 shadow-sm border text-sm flex items-center justify-center"
  >
    {{ label }}
  </div>
  <Select
    v-if="!readonly"
    v-bind="$attrs"
    :model-value="modelValue"
    @update:model-value="emit('update:model-value', $event as string)"
    :required="required"
  >
    <slot />
  </Select>
</template>

<script setup lang="ts">
import { Select } from '@/components/ui/select'

defineOptions({
  inheritAttrs: false,
})

defineProps<{
  readonly: boolean
  label: string
  modelValue: string
  required?: boolean
}>()

const emit = defineEmits<{
  'update:model-value': [value: string]
}>()
</script>
