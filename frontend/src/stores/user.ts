import { type Team, type User, UserSchema } from '@/types.ts'
import { computed, ref, watch } from 'vue'
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

    localStorage.setItem('userStore', JSON.stringify({ token: token.value }))
  }

  function logOut() {
    token.value = null
    user.value = null
    team.value = null

    localStorage.removeItem('userStore')
  }

  watch(token, () => {
    validateToken()
  })

  function validateToken() {
    if (!token.value) {
      return
    }
    const parts = token.value.split('.')
    if (parts.length !== 3) {
      token.value = null
      return
    }
    try {
      const payload = JSON.parse(atob(parts[1]))
      const expiry = new Date(payload['exp'] * 1000)
      const secondsToExpiry = (expiry.getTime() - new Date().getTime()) / 1000
      if (secondsToExpiry < 15 * 60) {
        console.log(
          'Your login token expires in less than 15 minutes, logging out',
          secondsToExpiry,
        )
        token.value = null
      }
    } catch (e) {
      console.log('Failed to parse login token', e)
      token.value = null
    }
  }

  return { accountReady, token, team, user, loggedIn, logIn, logOut, validateToken }
})

export function hydrateUserStore() {
  const store = useUserStore()
  const existingItem = localStorage.getItem('userStore')
  if (existingItem) {
    const parsed = JSON.parse(existingItem)
    store.token = parsed.token
  }
}
