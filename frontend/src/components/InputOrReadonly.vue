<template>
  <div
    v-if="readonly"
    v-bind="$attrs"
    class="self-stretch shrink-0 rounded-md h-7 px-2 shadow-sm border text-sm flex items-center justify-center"
  >
    {{ label }}
  </div>
  <Input
    v-if="!readonly"
    v-bind="$attrs"
    type="number"
    :placeholder="label"
    :model-value="modelValue"
    @update:model-value="emit('update:model-value', $event as number)"
  />
</template>

<script setup lang="ts">
import { Input } from '@/components/ui/input'

defineOptions({
  inheritAttrs: false,
})

defineProps<{
  readonly: boolean
  label: string
  modelValue: number
  required?: boolean
}>()

const emit = defineEmits<{
  'update:model-value': [value: number]
}>()
</script>
