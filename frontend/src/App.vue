<template>
  <div>
    <Toaster :rich-colors="true" position="top-center" class="pointer-events-auto" />
    <NavBar />
    <div v-if="isFetching" class="h-1 w-full absolute bg-gradient-primary" />
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
import { useUserStore } from '@/stores/user.ts'
import { watch } from 'vue'

const { data: myself, isFetching } = queryMyself()
const { team, user, accountReady } = storeToRefs(useUserStore())
const route = useRoute()

watch(myself, (newData) => {
  if (newData) {
    team.value = newData.team
    user.value = newData.user
  }
})
</script>
