import { type Team, type User, UserSchema } from '@/types.ts'
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { fetchWithError } from '@/data/fetching.ts'

export const useUserStore = defineStore('user', () => {
  const token = ref<string | null>(null)
  const user = ref<User | null>(null)
  const team = ref<Team | null>(null)

  const loggedIn = computed(() => token.value !== null)

  async function logIn() {
    const res = await fetchWithError(`/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'admin', password: 'admin' }),
    })
    const json = await res.json()
    user.value = UserSchema.parse(json['user'])
    token.value = json['token']
  }

  return { token, team, user, loggedIn, logIn }
})
