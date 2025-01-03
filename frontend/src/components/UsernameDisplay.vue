<template>
  <span
    class="font-medium"
    :class="[
      me?.id === id
        ? 'animate-gradient-x bg-gradient-to-r from-blue-500 via-violet-500 to-rose-600 bg-clip-text text-transparent'
        : '',
    ]"
  >
    {{ displayName }}
  </span>
  <span class="text-muted-foreground"> ({{ id }})</span>
</template>

<script setup lang="ts">
import type { UserId } from '@/types.ts'
import { storeToRefs } from 'pinia'
import { toRefs } from 'vue'
import { useUserStore } from '@/stores/user.ts'

const { user: me } = storeToRefs(useUserStore())

const props = withDefaults(
  defineProps<{
    id: UserId
    displayName: string
    showId: boolean
  }>(),
  {
    showId: false,
  },
)
const { id, displayName } = toRefs(props)
</script>
