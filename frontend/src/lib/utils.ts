import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

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
    return `${days}d ${hours % 24}h`
  } else if (hours > 0) {
    return `${hours}h ${minutes % 60}m`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  } else if (seconds > 0) {
    return `${seconds}s`
  } else {
    return `${millis}ms`
  }
}

export function formatApproxDuration(currentTime: number, insertTime: number) {
  return formatDuration(Math.floor((currentTime - insertTime) / 5000) * 5000)
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
