import {
  type FinishedCompilerTask,
  FinishedCompilerTaskSchema,
  type FinishedCompilerTaskSummary,
  FinishedCompilerTaskSummarySchema,
  type IntegrationInfoResponse,
  IntegrationInfoResponseSchema,
  type ListTestResponse,
  ListTestResponseSchema,
  type QueueResponse,
  QueueResponseSchema,
  type Repo,
  RepoSchema,
  type RequestRevision,
  RequestRevisionSchema,
  type SetTestResponse,
  SetTestResponseSchema,
  type ShowMyselfResponse,
  ShowMyselfResponseSchema,
  type TaskId,
  type TeamId,
  TeamIdSchema,
  type TeamInfo,
  TeamInfoSchema,
  type TestId,
  type TestWithTestTasting,
  TestWithTestTastingSchema,
  type WorkItem,
  WorkItemSchema,
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
}

type TestPatch = {
  input: string
  id: TestId
  expectedOutput: string
  category: string
  ignoreTestTasting: boolean
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

async function fetchRepo(team: TeamId): Promise<Repo | null> {
  const response = await fetchWithAuth(`/repo/${encodeURIComponent(team)}`, undefined, {
    extraSuccessStatus: [404],
  })
  if (response.status === 404) {
    return null
  }
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

export async function fetchGetRecentTasks(count: number): Promise<FinishedCompilerTaskSummary[]> {
  const response = await fetchWithAuth(`/team/recent-tasks/${count}`)
  const json = await response.json()
  return z.array(FinishedCompilerTaskSummarySchema).parse(json)
}

export function queryRecentTasks(count?: number) {
  return useQuery({
    queryKey: ['recent-tasks'],
    queryFn: () => fetchGetRecentTasks(count == undefined ? 10 : count),
    refetchInterval: 2 * 60 * 1000, // 2 minutes
    meta: {
      purpose: 'fetching recent tasks',
    },
    enabled: isLoggedIn(),
  })
}

export async function fetchTask(taskId: TaskId): Promise<FinishedCompilerTask | null> {
  const response = await fetchWithAuth(`/tasks/${encodeURIComponent(taskId)}`, undefined, {
    extraSuccessStatus: [404],
  })
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

export async function fetchTests(): Promise<ListTestResponse> {
  const response = await fetchWithAuth('/tests')
  const json = await response.json()
  return ListTestResponseSchema.parse(json)
}

export function queryTests() {
  return useQuery({
    queryKey: ['tests'],
    queryFn: fetchTests,
    refetchInterval: 2 * 60 * 1000, // 2 minutes
    meta: {
      purpose: 'fetching test summaries',
    },
    enabled: isLoggedIn(),
  })
}

export async function fetchTestDetail(testId: TestId): Promise<TestWithTestTasting | null> {
  const response = await fetchWithAuth(`/tests/${encodeURIComponent(testId)}`)
  if (response.status === 404) {
    return null
  }
  const json = await response.json()
  return TestWithTestTastingSchema.parse(json)
}

export function queryTest(testId: MaybeRefOrGetter<TestId | undefined>, refetchOnMount?: boolean) {
  const enabled = computed(() => !!toRef(testId).value)
  const loggedIn = isLoggedIn()

  return useQuery({
    queryKey: ['tests', testId],
    queryFn: () => fetchTestDetail(toValue(testId)!),
    enabled: computed(() => enabled.value && loggedIn.value),
    refetchOnMount,
    meta: {
      purpose: 'fetching test details',
    },
  })
}

export async function fetchSetTest(test: TestPatch): Promise<SetTestResponse> {
  const response = await fetchWithAuth(`/tests/${encodeURIComponent(test.id)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      input: test.input,
      expectedOutput: test.expectedOutput,
      category: test.category,
      ignoreTestTasting: test.ignoreTestTasting,
    }),
  })
  const json = await response.json()
  return SetTestResponseSchema.parse(json)
}

export function mutateTest(queryClient: QueryClient) {
  return useMutation({
    mutationFn: (test: TestPatch) => fetchSetTest(test),
    onSuccess: (_, args, __) => {
      const ___ = queryClient.invalidateQueries({ queryKey: ['tests', args.id] })
      const ____ = queryClient.invalidateQueries({ queryKey: ['tests'] })
    },
    meta: {
      purpose: 'updating a test',
    },
  })
}

export async function fetchDeleteTest(testId: TestId): Promise<boolean> {
  const response = await fetchWithAuth(`/tests/${encodeURIComponent(testId)}`, {
    method: 'DELETE',
  })
  return response.ok
}

export function mutateDeleteTest(queryClient: QueryClient) {
  return useMutation({
    mutationFn: (testId: TestId) => fetchDeleteTest(testId),
    onSuccess: (_, args, __) => {
      const ___ = queryClient.invalidateQueries({ queryKey: ['tests', args] })
      const ____ = queryClient.invalidateQueries({ queryKey: ['tests'] })
    },
    meta: {
      purpose: 'deleting a test',
    },
  })
}

export async function fetchQueue(): Promise<QueueResponse> {
  const response = await fetchWithAuth('/queue')
  const json = await response.json()
  return QueueResponseSchema.parse(json)
}

export function queryQueue(refetchIntervalMs: number) {
  return useQuery({
    queryKey: ['queue'],
    queryFn: fetchQueue,
    refetchInterval: refetchIntervalMs,
    staleTime: 1000,
    meta: {
      purpose: 'fetching the queue',
    },
    enabled: isLoggedIn(),
  })
}

export async function fetchRunningTaskExists(taskId: TaskId): Promise<boolean> {
  const response = await fetchWithAuth(
    `/tasks/${encodeURIComponent(taskId)}/stream`,
    {
      method: 'HEAD',
    },
    { extraSuccessStatus: [404] },
  )
  return response.status === 200
}

export async function fetchTaskExists(taskId: TaskId): Promise<boolean> {
  const response = await fetchWithAuth(
    `/tasks/${encodeURIComponent(taskId)}`,
    {
      method: 'HEAD',
    },
    { extraSuccessStatus: [404] },
  )
  return response.status === 200
}

export async function fetchQueuedTask(taskId: TaskId): Promise<WorkItem | null> {
  const response = await fetchWithAuth(`/queue/task/${encodeURIComponent(taskId)}`, undefined, {
    extraSuccessStatus: [404],
  })
  if (response.status === 404) {
    return null
  }
  const json = await response.json()
  return WorkItemSchema.parse(json)
}

export async function fetchRequestRevision(revision: string): Promise<RequestRevision | null> {
  const response = await fetchWithAuth(
    `/queue/rev/${encodeURIComponent(revision)}`,
    {
      method: 'PUT',
    },
    { extraSuccessStatus: [404] },
  )
  if (response.status === 404) {
    return null
  }
  return RequestRevisionSchema.parse(await response.json())
}

export function mutateRequestRevision(queryClient: QueryClient) {
  return useMutation({
    mutationFn: (revision: string) => fetchRequestRevision(revision),
    onSuccess: (_, __, ___) => {
      const ____ = queryClient.invalidateQueries({ queryKey: ['queue'] })
    },
    meta: {
      purpose: 'requesting a revision',
    },
  })
}

export async function fetchIntegrationStatus(): Promise<IntegrationInfoResponse> {
  const response = await fetchWithAuth('/users/me/integrations', undefined)
  return IntegrationInfoResponseSchema.parse(await response.json())
}

export function queryIntegrationStatus(_teamId: MaybeRefOrGetter<TeamId | undefined>) {
  const enabled = computed(() => !!toRef(_teamId).value)
  const loggedIn = isLoggedIn()

  return useQuery({
    queryKey: ['integrations', _teamId],
    queryFn: fetchIntegrationStatus,
    meta: {
      purpose: 'fetching integration status',
    },
    enabled: computed(() => enabled.value && loggedIn.value),
  })
}

export async function fetchTopTaskPerTeam(): Promise<Map<TeamId, FinishedCompilerTaskSummary>> {
  const response = await fetchWithAuth('/top-tasks')
  const result = new Map()
  for (const [k, v] of Object.entries(await response.json())) {
    result.set(TeamIdSchema.parse(k), FinishedCompilerTaskSummarySchema.parse(v))
  }
  return result
}

export function queryTopTaskPerTeam() {
  return useQuery({
    queryKey: ['top-task'],
    queryFn: fetchTopTaskPerTeam,
    meta: {
      purpose: 'fetching top task per team',
    },
    enabled: isLoggedIn(),
  })
}
