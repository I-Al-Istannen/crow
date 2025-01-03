import {
  type FinishedCompilerTask,
  FinishedCompilerTaskSchema,
  type FinishedCompilerTaskSummary,
  FinishedCompilerTaskSummarySchema,
  type Repo,
  RepoSchema,
  type ShowMyselfResponse,
  ShowMyselfResponseSchema,
  type TaskId,
  type TeamId,
  type TeamInfo,
  TeamInfoSchema,
} from '@/types.ts'
import { QueryClient, useMutation, useQuery } from '@tanstack/vue-query'
import { type Ref, computed, toRef, toValue } from 'vue'
import type { MaybeRefOrGetter } from '@vueuse/core'
import { fetchWithAuth } from '@/data/fetching.ts'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user.ts'
import { z } from 'zod'

type RepoPatch = {
  repoUrl: string
  autoFetch: boolean
}

async function fetchMyself(): Promise<ShowMyselfResponse> {
  const response = await fetchWithAuth('/users/me')
  const json = await response.json()
  return ShowMyselfResponseSchema.parse(json)
}

function isLoggedIn(): Ref<boolean> {
  const { loggedIn } = storeToRefs(useUserStore())
  return loggedIn
}

export function queryMyself() {
  return useQuery({
    queryKey: ['userinfo'],
    queryFn: fetchMyself,
    refetchInterval: 10 * 60 * 1000, // 10 minutes
    meta: {
      purpose: 'fetching user information',
    },
    enabled: isLoggedIn(),
  })
}

async function fetchRepo(team: TeamId): Promise<Repo> {
  const response = await fetchWithAuth(`/repo/${encodeURIComponent(team)}`)
  const json = await response.json()
  return RepoSchema.parse(json)
}

export function queryRepo(team: MaybeRefOrGetter<TeamId | undefined>) {
  const enabled = computed(() => !!toRef(team).value)
  return useQuery({
    queryKey: ['repo', team],
    queryFn: () => fetchRepo(toValue(team)!),
    meta: {
      purpose: 'fetching your repository',
    },
    enabled: computed(() => enabled.value && isLoggedIn().value),
  })
}

async function fetchSetRepo(team: TeamId, repo: RepoPatch): Promise<Repo> {
  const response = await fetchWithAuth(`/repo/${encodeURIComponent(team)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(repo),
  })
  const json = await response.json()
  return RepoSchema.parse(json)
}

export function mutateRepo(queryClient: QueryClient) {
  return useMutation({
    mutationFn: ([team, repo]: [TeamId, RepoPatch]) => fetchSetRepo(team, repo),
    onSuccess: (_, args, __) => {
      const ___ = queryClient.invalidateQueries({ queryKey: ['repo', args[0]] })
    },
    meta: {
      purpose: 'updating your repository',
    },
  })
}

export async function fetchGetRecentTasks(): Promise<FinishedCompilerTaskSummary[]> {
  const response = await fetchWithAuth('/team/recent-tasks')
  const json = await response.json()
  return z.array(FinishedCompilerTaskSummarySchema).parse(json)
}

export function queryRecentTasks() {
  return useQuery({
    queryKey: ['recent-tasks'],
    queryFn: fetchGetRecentTasks,
    refetchInterval: 2 * 60 * 1000, // 2 minutes
    meta: {
      purpose: 'fetching recent tasks',
    },
    enabled: isLoggedIn(),
  })
}

export async function fetchTask(taskId: TaskId): Promise<FinishedCompilerTask | null> {
  const response = await fetchWithAuth(`/tasks/${encodeURIComponent(taskId)}`)
  if (response.status === 404) {
    return null
  }
  const json = await response.json()
  return FinishedCompilerTaskSchema.parse(json)
}

export function queryTask(taskId: MaybeRefOrGetter<TaskId | undefined>) {
  const enabled = computed(() => !!toRef(taskId).value)
  const loggedIn = isLoggedIn()

  return useQuery({
    queryKey: ['task', taskId],
    queryFn: () => fetchTask(toValue(taskId)!),
    enabled: computed(() => enabled.value && loggedIn.value),
    meta: {
      purpose: 'fetching task details',
    },
  })
}

export async function fetchTeamInfo(teamId: TeamId): Promise<TeamInfo | null> {
  const response = await fetchWithAuth(`/team/info/${encodeURIComponent(teamId)}`)
  if (response.status === 404) {
    return null
  }
  const json = await response.json()
  return TeamInfoSchema.parse(json)
}

export function queryTeamInfo(teamId: MaybeRefOrGetter<TeamId | undefined>) {
  const enabled = computed(() => !!toRef(teamId).value)
  const loggedIn = isLoggedIn()

  return useQuery({
    queryKey: ['team', teamId],
    queryFn: () => fetchTeamInfo(toValue(teamId)!),
    enabled: computed(() => enabled.value && loggedIn.value),
    meta: {
      purpose: 'fetching team information',
    },
  })
}
