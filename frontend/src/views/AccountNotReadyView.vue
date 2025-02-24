<template>
  <PageContainer>
    <Card>
      <CardHeader>
        <CardTitle>Hey there, traveller</CardTitle>
        <CardDescription v-if="!loggedIn">Sadly, crow requires authentication.</CardDescription>
        <CardDescription v-else>You need to be part of a team</CardDescription>
      </CardHeader>
      <CardContent>
        <a :href="loginUrl" v-if="!loggedIn">
          <Button v-if="!loggedIn">Log in</Button>
        </a>
        <div v-else>
          It seems like you are not yet part of a team :)
          <br />
          You will be assigned by the course administrators. If you believe this is an error, feel
          encouraged to report it!
        </div>
      </CardContent>
    </Card>
  </PageContainer>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { BACKEND_URL } from '@/data/fetching.ts'
import { Button } from '@/components/ui/button'
import PageContainer from '@/components/PageContainer.vue'
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'

const { loggedIn } = storeToRefs(useUserStore())

const loginUrl = computed(() => BACKEND_URL + '/login')
</script>
