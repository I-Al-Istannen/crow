import { type Team, type User, UserSchema } from '@/types.ts'
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { fetchWithError } from '@/data/fetching.ts'

export const useUserStore = defineStore('user', () => {
  const token = ref<string | null>(null)
  const user = ref<User | null>(null)
  const team = ref<Team | null>(null)

  const loggedIn = computed(() => token.value !== null)
  const accountReady = computed(() => loggedIn.value && team.value !== null)

  async function logIn(oidc_code: string, oidc_state: string) {
    const res = await fetchWithError(
      `/login/oidc/callback?code=${encodeURIComponent(oidc_code)}&state=${encodeURIComponent(oidc_state)}`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code: oidc_code, state: oidc_state }),
        credentials: 'include',
      },
    )
    const json = await res.json()
    user.value = UserSchema.parse(json['user'])
    token.value = json['token']
  }

  return { accountReady, token, team, user, loggedIn, logIn }
})
