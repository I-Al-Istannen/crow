import { type ClassValue, clsx } from 'clsx'
import { computed, toRef } from 'vue'
import { type MaybeRefOrGetter } from '@vueuse/core'
import type { TeamId } from '@/types.ts'
import { queryRepo } from '@/data/network.ts'
import { storeToRefs } from 'pinia'
import { twMerge } from 'tailwind-merge'
import { useUserStore } from '@/stores/user.ts'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatTime(date: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  }).format(date)
}

export function formatDurationBetween(start: Date, end: Date): string {
  return formatDuration(end.getTime() - start.getTime())
}

export function formatDuration(millis: number): string {
  const seconds = Math.floor(millis / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) {
    return `${days.toString()}d ${(hours % 24).toString()}h`
  } else if (hours > 0) {
    return `${hours.toString()}h ${(minutes % 60).toString()}m`
  } else if (minutes > 0) {
    return `${minutes.toString()}m ${(seconds % 60).toString()}s`
  } else if (seconds > 0) {
    return `${seconds.toString()}s`
  } else {
    return `${millis.toString()}ms`
  }
}

export function formatApproxDuration(currentTime: number, insertTime: number) {
  const delta = Math.max(currentTime - insertTime, 0)
  return formatDuration(Math.floor(delta / 1000) * 1000)
}

export function formatBusyDuration(currentTime: number, insertTime: number) {
  const millis = Math.max(currentTime - insertTime, 0)
  if (millis < 10 * 1000) {
    return `${Math.floor(millis / 1000).toString()}s`
  }

  const seconds = Math.floor(millis / 1000)
  if (seconds < 60) {
    return `> ${(Math.floor(seconds / 10) * 10).toString()}s`
  }

  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) {
    return `> ${Math.floor(minutes).toString()}m`
  }

  return `Since ${new Date(insertTime).toLocaleString()}`
}

export function statusColor(
  status: 'Success' | 'Failure' | 'Error' | 'Timeout' | 'Aborted' | 'Queued' | 'Started',
  prefix: string,
): string {
  switch (status) {
    // comments are here to make tailwind pick up and generate the classes
    case 'Success':
      // bg-green-500
      // text-green-500
      return prefix + '-green-500'
    case 'Error':
      // bg-red-400
      // text-red-400
      return prefix + '-red-400'
    case 'Failure':
      // bg-red-500
      // text-red-500
      return prefix + '-red-500'
    case 'Timeout':
      // bg-orange-500
      // text-orange-500
      return prefix + '-orange-500'
    case 'Aborted':
      // bg-gray-500
      // text-gray-500
      return prefix + '-gray-500'
    case 'Queued':
      // bg-gray-500
      // text-gray-500
      return prefix + '-gray-500'
    case 'Started':
      // bg-blue-500
      // text-blue-500
      return prefix + '-blue-500'
  }
}

export function useCommitUrl(
  commitGet: MaybeRefOrGetter<string>,
  teamGet: MaybeRefOrGetter<TeamId>,
  repoUrlGet: MaybeRefOrGetter<string | undefined>,
) {
  const commit = toRef(commitGet)
  const repoUrl = toRef(repoUrlGet)
  const commitTeam = toRef(teamGet)
  const { team: myTeam } = storeToRefs(useUserStore())
  const { data: repo } = queryRepo(computed(() => myTeam.value?.id))
  const finalRepoUrl = computed(() => {
    if (repoUrl.value) {
      return repoUrl.value
    }
    if (myTeam.value?.id === commitTeam.value) {
      return repo.value?.url
    }
    return undefined
  })

  const commitUrl = computed(() => {
    const url = finalRepoUrl.value
    if (!url) {
      return undefined
    }

    const match = /github\.com[/:](?<user>[^/\n]+)\/(?<repo>[^/\n]+)(\.git)?/.exec(url)
    if (!match?.groups) {
      return undefined
    }

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return `https://github.com/${match.groups.user!}/${match.groups.repo!}/commit/${commit.value}`
  })

  return { commitUrl }
}
