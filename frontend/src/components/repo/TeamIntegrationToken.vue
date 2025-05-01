<template>
  <div>
    <span>Your Team integration token is: </span>
    <code :class="cls" class="select-all">{{ teamIntegrationToken }}</code>
    <div class="mt-2 leading-relaxed">
      With this token you can access two special endpoints:
      <ul class="list-disc list-inside ml-2 mb-1">
        <li>
          <code :class="cls">PUT /integration/token/queue/rev/:revision</code>
          to submit a commit of your repository to the queue. Optionally, you can set the content
          type to <code :class="cls">application/json</code> and provide a commit message in the
          body, like so <code :class="cls">{ "commitMessage": "msg"}</code>. This message will be
          used instead of the commit message then.
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
        Here is a sample curl command for getting the status:<br />
        <code :class="cls" class="select-all">{{ curlStatus }}</code>
        <br />
        <br />
        Here is a sample curl command for queueing a task:<br />
        <code :class="cls" class="select-all">{{ curlQueue }}</code>
        <br />
        <br />
        Here is a sample curl command for queueing a task with the message
        <code :class="cls">"Foo"</code>:<br />
        <code :class="cls" class="select-all">{{ curlQueueMessage }}</code>
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

const curlStatus = computed(
  () =>
    `curl --header 'Authorization: Bearer ${teamIntegrationToken.value}'` +
    ` ${BACKEND_URL}/integration/token/task/:task_id`,
)
const curlQueue = computed(
  () =>
    `curl -X PUT --header 'Authorization: Bearer ${teamIntegrationToken.value}'` +
    ` ${BACKEND_URL}/integration/token/queue/rev/:revision`,
)
const curlQueueMessage = computed(
  () =>
    `curl -X PUT --header 'Authorization: Bearer ${teamIntegrationToken.value}'` +
    `  --header 'Content-Type: application/json' ${BACKEND_URL}/integration/token/queue/rev/:revision` +
    ` --data '{ "commitMessage": "Foo" }'`,
)
</script>
