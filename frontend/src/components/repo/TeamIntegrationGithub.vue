<template>
  <p>
    You can
    <a class="cursor-pointer underline" :href="appUrl" target="_blank">
      add our GitHub app to your repository</a
    >, which automatically mirrors the queued, running and finish status from crow to commits right
    in your repository. Crow uses GitHub's Commit Status API, allowing you to require green crow
    results for merging PRs.
  </p>
  <p class="mt-2">
    You still need to queue commits you are interested in manually, but the app will suggest a
    sample GitHub Actions workflow to do this automatically.
  </p>
  <p class="mt-2">
    If you don't want to authorize the GitHub App, you can use the integration token with our
    Quickstart action instead.
  </p>
</template>

<script setup lang="ts">
import { toRefs } from 'vue'

const props = defineProps<{
  appUrl: string
}>()
const { appUrl } = toRefs(props)
</script>
