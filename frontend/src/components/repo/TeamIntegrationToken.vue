<template>
  <div>
    <span>Your Team integration token is: </span>
    <code :class="cls" class="select-all">{{ teamIntegrationToken }}</code>
    <div class="mt-2 leading-relaxed">
      With this token you can access two special endpoints:
      <ul class="list-disc list-inside ml-2 mb-1">
        <li>
          <code :class="cls">POST /integration/token/queue/rev/:revision</code>
          to submit a commit of your repository to the queue
        </li>
        <li>
          <code :class="cls">GET /integration/token/task/:task_id</code>
          to check the status of a given task
        </li>
      </ul>
      <p>
        The task status will be one of the following:
        <code :class="cls">Queued</code>, <code :class="cls">Running</code>,
        <code :class="cls">Error</code>, <code :class="cls">Timeout</code>,
        <code :class="cls">Aborted</code>, or <code :class="cls">Success</code>.
      </p>
      <p>
        The endpoints use
        <code :class="cls">Bearer</code> authentication, so you need to send the token in the
        <code :class="cls">Authorization</code> header:<br />
        <code :class="cls">Authorization: Bearer {{ teamIntegrationToken }}</code>
      </p>
      <p class="mt-4">
        Here is a sample curl command:<br />
        <code :class="cls" class="select-all">{{ curl }}</code>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, toRefs } from 'vue'
import { BACKEND_URL } from '@/data/fetching.ts'

const cls = 'text-sm bg-accent p-1 rounded-md'

const props = defineProps<{
  teamIntegrationToken: string
}>()
const { teamIntegrationToken } = toRefs(props)

const curl = computed(
  () =>
    `curl --header 'Authorization: Bearer ${teamIntegrationToken.value}'` +
    ` ${BACKEND_URL}/integration/token/task/:task_id`,
)
</script>
