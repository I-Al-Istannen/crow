import { createRouter, createWebHistory } from 'vue-router'
import AllTasksView from '@/views/AllTasksView.vue'
import HomeView from '@/views/HomeView.vue'
import QueueView from '@/views/QueueView.vue'
import RepoView from '@/views/RepoView.vue'
import TaskDetailView from '@/views/TaskDetailView.vue'
import TeamInfoView from '@/views/TeamInfoView.vue'
import TestView from '@/views/TestView.vue'
import LoginCallbackView from '@/views/LoginCallbackView.vue'

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
      path: '/tests',
      name: 'tests',
      component: TestView,
      meta: {
        name: 'Tests',
      },
    },
    {
      path: '/queue',
      name: 'queue',
      component: QueueView,
      meta: {
        name: 'Queue',
      },
    },
    {
      path: '/all-tasks',
      name: 'all-tasks',
      component: AllTasksView,
      meta: {
        name: 'All Tasks',
        hidden: true,
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
    {
      path: '/login/oidc-callback',
      name: 'oidc-callback',
      component: LoginCallbackView,
      meta: {
        name: 'Login callback',
        hidden: true,
      },
    },
  ],
})

export default router
