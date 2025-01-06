<script setup lang="ts">
import { ComboboxInput, type ComboboxInputProps, useForwardProps } from 'radix-vue'
import { type HTMLAttributes, computed, ref } from 'vue'
import { MagnifyingGlassIcon } from '@radix-icons/vue'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<
  ComboboxInputProps & {
    class?: HTMLAttributes['class']
  }
>()

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})

const forwardedProps = useForwardProps(delegatedProps)

const comboBox = ref<InstanceType<typeof ComboboxInput>>()

function focus() {
  comboBox.value?.$el?.focus()
}

function isFocused() {
  return comboBox.value?.$el === document.activeElement
}

defineExpose({ focus, isFocused })
</script>

<template>
  <div class="flex items-center border-b px-3" cmdk-input-wrapper>
    <MagnifyingGlassIcon class="mr-2 h-4 w-4 shrink-0 opacity-50" />
    <ComboboxInput
      v-bind="{ ...forwardedProps, ...$attrs }"
      auto-focus
      ref="comboBox"
      :class="
        cn(
          'flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50',
          props.class,
        )
      "
    />
  </div>
</template>
