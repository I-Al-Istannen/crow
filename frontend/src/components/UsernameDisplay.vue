<template>
  <span class="font-medium" :class="[me?.id === id ? 'gradient-primary' : '']">
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
