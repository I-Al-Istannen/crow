<template>
  <div>
    <Toaster :rich-colors="true" position="top-center" class="pointer-events-auto" />
    <NavBar />
    <div v-if="isFetching" class="bg-gradient-primary absolute h-1 w-full" />
    <RouterView v-if="route.name === 'oidc-callback' || accountReady" />
    <AccountNotReadyView v-else />
  </div>
</template>

<script setup lang="ts">
import { RouterView, useRoute } from 'vue-router'
import AccountNotReadyView from '@/views/AccountNotReadyView.vue'
import NavBar from '@/components/NavBar.vue'
import { Toaster } from '@/components/ui/sonner'
import { queryMyself } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { useTitle } from '@vueuse/core'
import { useUserStore } from '@/stores/user.ts'
import { watch } from 'vue'

const { data: myself, isFetching } = queryMyself()
const { team, user, accountReady } = storeToRefs(useUserStore())
const route = useRoute()

const pageTitle = useTitle(undefined, { restoreOnUnmount: false })

watch(
  route,
  (newRoute) => {
    if (newRoute.meta.managesTitle) {
      return
    }
    const title = (newRoute.meta.title ?? newRoute.meta.name) as string | undefined
    if (newRoute.meta.title as string | undefined) {
      pageTitle.value = title
    } else if (newRoute.meta.name as string | undefined) {
      pageTitle.value = `${title ?? ''} - crow`
    } else {
      pageTitle.value = 'crow'
    }
  },
  { immediate: true },
)

watch(myself, (newData) => {
  if (newData) {
    team.value = newData.team
    user.value = newData.user
  }
})
</script>
