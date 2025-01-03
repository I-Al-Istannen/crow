import './assets/index.css'
import './assets/fonts.css'

import { MutationCache, QueryCache, VueQueryPlugin } from '@tanstack/vue-query'
import App from './App.vue'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import { toast } from 'vue-sonner'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(VueQueryPlugin, {
  queryClientConfig: {
    queryCache: new QueryCache({
      onError: (error, query) => {
        toast.error('Error ' + (query.meta?.purpose || 'during request'), {
          description: error.message,
          duration: 5000,
        })
        console.log('Error during request', query, error)
      },
    }),
    mutationCache: new MutationCache({
      onError: (error, _vars, _context, mutation) => {
        toast.error('Error ' + (mutation.meta?.purpose || 'during mutation'), {
          description: error.message,
          duration: 5000,
        })
        console.log('Error during mutation', mutation, error)
      },
    }),
  },
})

app.mount('#app')
