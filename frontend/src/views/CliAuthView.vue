<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Authorize crow CLI</CardTitle>
        <CardDescription>Get a token to use with the crow cli</CardDescription>
      </CardHeader>
      <CardContent>
        Please enter the following token in the
        <code class="rounded-md bg-accent p-1 text-sm">crow-client login</code> command. <br />You
        can close this tab afterwards.
        <div
          class="relative mt-2 max-w-lg text-wrap break-all rounded-md bg-accent p-1 text-sm"
          :class="{ 'hover:bg-accent/65': !revealed }"
        >
          <span :class="{ 'text-transparent': !revealed }">{{ token }}</span>
          <div
            class="absolute left-0 top-0 flex h-full w-full cursor-pointer items-center justify-center"
            v-if="!revealed"
            @click="revealed = true"
          >
            Click to reveal
          </div>
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { computed, ref } from 'vue'
import PageContainer from '@/components/PageContainer.vue'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'

const revealed = ref(false)

const { token: actualToken } = storeToRefs(useUserStore())

const token = computed(() => {
  if (revealed.value || !actualToken.value) {
    return actualToken.value
  }
  return 'a'.repeat(actualToken.value.length)
})
</script>
