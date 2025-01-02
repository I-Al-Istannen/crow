<template>
  <div>
    <Toaster :rich-colors="true" position="top-center" class="pointer-events-auto" />
    <NavBar />
    <RouterView />
  </div>
</template>

<script setup lang="ts">
import NavBar from '@/components/NavBar.vue'
import { RouterView } from 'vue-router'
import { Toaster } from '@/components/ui/sonner'
import { onMounted } from 'vue'
import { toast } from 'vue-sonner'
import { useUserStore } from '@/stores/user.ts'

onMounted(async () => {
  toast.promise(useUserStore().logIn(), {
    loading: 'Loading...',
    success: 'Logged in!',
    error: 'Failed to log in.',
  })
})
</script>
