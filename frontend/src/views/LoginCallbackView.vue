<template>
  <PageContainer>
    <Card>
      <CardHeader v-if="error === null">
        <CardTitle>Logging in...</CardTitle>
        <CardDescription>Please stand by, logging you in to crow</CardDescription>
      </CardHeader>
      <CardHeader v-else>
        <CardTitle>Failed to log in</CardTitle>
        <CardDescription>
          Something went wrong while logging you in. Please report this error and
          <router-link class="text-black underline" to="/">go back.</router-link>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div v-if="pending">
          <LoaderCircle class="-ml-2 mr-2 animate-spin" />
        </div>
        <div v-else-if="error !== null">
          <div class="text-red-500">{{ error }}</div>
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { FetchError } from '@/data/fetching.ts'
import { LoaderCircle } from 'lucide-vue-next'
import { PRE_LOGIN_URL_SESSION_STORAGE_KEY } from '@/router'
import PageContainer from '@/components/PageContainer.vue'
import { useUserStore } from '@/stores/user.ts'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()

const pending = ref(false)
const error = ref<string | null>(null)

watch(
  route,
  async (route) => {
    const code = route.query['code'] as string
    const state = route.query['state'] as string

    pending.value = true
    error.value = null
    try {
      await userStore.logIn(code, state)
      const originalLocation = sessionStorage.getItem(PRE_LOGIN_URL_SESSION_STORAGE_KEY)
      await router.replace(originalLocation || '/')
    } catch (e) {
      if (e instanceof FetchError) {
        error.value = e.message
      } else {
        error.value = `Unknown error: ${e}`
      }
    } finally {
      pending.value = false
    }
  },
  { immediate: true },
)
</script>
