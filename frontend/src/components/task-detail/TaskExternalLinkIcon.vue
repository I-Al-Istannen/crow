<template>
  <a
    v-if="commitUrl && team?.id === teamId"
    :href="commitUrl"
    target="_blank"
    rel="noopener noreferrer nofollow"
    @click.prevent="openUrl(commitUrl)"
  >
    <LucideExternalLink class="inline h-4 w-4" />
  </a>
</template>

<script setup lang="ts">
import { LucideExternalLink } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { toRefs } from 'vue'
import { useCommitUrl } from '@/lib/utils.ts'
import { useUserStore } from '@/stores/user.ts'

const props = defineProps<{
  revision: string
  teamId: string
}>()
const { revision, teamId } = toRefs(props)

const { commitUrl } = useCommitUrl(revision, teamId, undefined)
const { team } = storeToRefs(useUserStore())

function openUrl(url: string) {
  window.open(url, '_blank')
}
</script>
