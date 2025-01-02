<template>
  <div>
    <Toaster :rich-colors="true" position="top-center" class="pointer-events-auto" />
    <NavBar />
    <RouterView />
  </div>
</template>

<script setup lang="ts">
import { onMounted, watch } from 'vue'
import NavBar from '@/components/NavBar.vue'
import { RouterView } from 'vue-router'
import { Toaster } from '@/components/ui/sonner'
import { queryMyself } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'

const { data: myself } = queryMyself()
const { team, user } = storeToRefs(useUserStore())

watch(myself, (newData) => {
  if (newData) {
    team.value = newData.team
    user.value = newData.user
  }
})

onMounted(() => {
  toast.promise(useUserStore().logIn(), {
    loading: 'Loading...',
    success: 'Logged in!',
    error: 'Failed to log in.',
  })
})
</script>
