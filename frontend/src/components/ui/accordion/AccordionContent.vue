<script setup lang="ts">
import { AccordionContent, type AccordionContentProps } from 'reka-ui'
import { type HTMLAttributes, computed, ref } from 'vue'
import { useMutationObserver, useResizeObserver } from '@vueuse/core'
import { cn } from '@/lib/utils'

const props = defineProps<AccordionContentProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})

// Slightly complicated machinery to update the animation target value when the content
// renders/changes its height after the initial mount.
const contentRef = ref<typeof AccordionContent | null>(null)
const element = computed(() => contentRef.value?.$el)
const contentDiv = ref<HTMLElement | null>(null)

useMutationObserver(
  element,
  () => {
    const children = element.value?.children
    if (children && children.length > 0) {
      contentDiv.value = children[0]
    }
  },
  { childList: true },
)

useResizeObserver(contentDiv, () => {
  // restart animation
  element.value!.classList.remove('data-[state=open]:animate-accordion-down')
  element.value!.style.setProperty(
    '--reka-collapsible-content-height',
    `${contentDiv.value!.clientHeight}px`,
  )
  element.value!.classList.add('data-[state=open]:animate-accordion-down')
})
</script>

<template>
  <AccordionContent
    v-bind="delegatedProps"
    ref="contentRef"
    class="overflow-hidden text-sm data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down"
  >
    <div :class="cn('pb-4 pt-0', props.class)">
      <slot />
    </div>
  </AccordionContent>
</template>
