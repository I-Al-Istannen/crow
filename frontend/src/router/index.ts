import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'
import RepoView from '@/views/RepoView.vue'
import TaskDetailView from '@/views/TaskDetailView.vue'
import TeamInfoView from '@/views/TeamInfoView.vue'

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
      path: '/repo',
      name: 'repo',
      component: RepoView,
      meta: {
        name: 'Repository',
      },
    },
    {
      path: '/task-detail/:taskId',
      name: 'task-detail',
      component: TaskDetailView,
      meta: {
        name: 'Task Detail',
        hidden: true,
      },
    },
    {
      path: '/team-info/:teamId',
      name: 'team-info',
      component: TeamInfoView,
      meta: {
        name: 'Team Info',
        hidden: true,
      },
    },
  ],
})

export default router
