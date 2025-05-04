<template>
  <div
    v-if="readonly"
    v-bind="$attrs"
    class="self-stretch shrink-0 rounded-md h-7 px-2 shadow-sm border text-sm flex items-center justify-start"
  >
    <span class="ml-1">{{ label }}</span>
  </div>
  <slot v-bind="$attrs" v-if="!readonly" />
</template>

<script setup lang="ts">
defineOptions({
  inheritAttrs: false,
})

defineProps<{
  readonly: boolean
  label: string
}>()
</script>
