import './assets/index.css'
import './assets/fonts.css'

import { MutationCache, QueryCache, VueQueryPlugin } from '@tanstack/vue-query'
import { hydrateUserStore, useUserStore } from '@/stores/user.ts'
import App from './App.vue'
import { FetchError } from '@/data/fetching.ts'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import { toast } from 'vue-sonner'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(VueQueryPlugin, {
  queryClientConfig: {
    defaultOptions: {
      queries: {
        retry: (_, error) => {
          if (error instanceof FetchError && error.status === 401) {
            useUserStore().validateToken()
          }
          return true
        },
      },
    },
    queryCache: new QueryCache({
      onError: (error, query) => {
        toast.error('Error ' + ((query.meta?.purpose as string | undefined) ?? 'during request'), {
          description: error.message,
          duration: 5000,
        })
        console.log('Error during request', query, error)
      },
    }),
    mutationCache: new MutationCache({
      onError: (error, _vars, _context, mutation) => {
        toast.error(
          'Error ' + ((mutation.meta?.purpose as string | undefined) ?? 'during mutation'),
          {
            description: error.message,
            duration: 5000,
          },
        )
        console.log('Error during mutation', mutation, error)
      },
    }),
  },
})

hydrateUserStore()

app.mount('#app')
