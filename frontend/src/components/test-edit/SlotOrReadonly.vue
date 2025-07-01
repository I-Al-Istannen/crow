<template>
  <div
    v-if="readonly"
    v-bind="$attrs"
    class="flex h-7 shrink-0 items-center justify-start self-stretch rounded-md border px-2 text-sm shadow-sm"
  >
    <span>{{ label }}</span>
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
