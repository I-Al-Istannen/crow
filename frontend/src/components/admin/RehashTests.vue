<template>
  <Card>
    <CardHeader>
      <CardTitle>Rehash Tests</CardTitle>
      <CardDescription>
        Rehash all tests to ensure their hash is consistent again after schema changes
      </CardDescription>
    </CardHeader>
    <CardContent>
      <Button :disabled="rehashPending" @click="rehash">Rehash tests</Button>
      <div class="mt-4 text-destructive" v-if="rehashError">
        Rehash failed
        <br />
        {{ rehashError }}
      </div>
    </CardContent>
  </Card>
</template>

<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { mutateRehashTests } from '@/data/network.ts'
import { toast } from 'vue-sonner'

const { mutateAsync: doRehash, isPending: rehashPending, error: rehashError } = mutateRehashTests()

async function rehash() {
  await doRehash()
  toast.success('Rehashed tests')
}
</script>
