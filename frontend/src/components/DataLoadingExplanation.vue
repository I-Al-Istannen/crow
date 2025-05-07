<template>
  <div v-if="isLoading" v-bind="$attrs" class="text-muted-foreground">
    <span>Loading</span>
    <LucideLoaderCircle :size="16" class="animate-spin inline ml-2" />
  </div>
  <div v-if="failureCount > 0" v-bind="$attrs" class="text-muted-foreground">
    <span class="text-red-500 opacity-80">
      Loading failed {{ failureCount }} time{{ failureCount > 1 ? 's' : '' }}.
    </span>
    <span class="text-black">crow</span> will retry periodically or you can refresh the page.
    <br />
    The last failure was
    <code class="text-sm bg-accent p-1 rounded-md">{{ failureReason }}</code>
  </div>
</template>

<script setup lang="ts">
import { LucideLoaderCircle } from 'lucide-vue-next'
import { toRefs } from 'vue'

defineOptions({
  inheritAttrs: false,
})

const props = defineProps<{
  isLoading: boolean
  failureCount: number
  failureReason: Error | null
}>()
const { isLoading, failureCount, failureReason } = toRefs(props)
</script>
