import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'
import RepoView from '@/views/RepoView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeView,
      meta: {
        name: 'Home',
      },
    },
    {
      path: '/',
      name: 'foobar',
      component: HomeView,
      meta: {
        name: 'Foobar',
      },
    },
    {
      path: "/repo",
      name: "repo",
      component: RepoView,
      meta: {
        name: "Repository",
      }
    }
  ],
})

export default router
