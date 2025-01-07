import { useUserStore } from '@/stores/user.ts'

export const BACKEND_URL = 'http://localhost:3000'

export class FetchError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.status = status
  }
}

export async function fetchWithError(
  url: string,
  init?: RequestInit,
  extra?: {
    extraSuccessStatus: number[]
  },
): Promise<Response> {
  if (!url.startsWith('http')) {
    url = BACKEND_URL + (url.startsWith('/') ? '' : '/') + url
  }
  const response = await fetch(url, init)
  if (
    !response.ok &&
    !(extra?.extraSuccessStatus && extra.extraSuccessStatus.includes(response.status))
  ) {
    throw new FetchError(`Failed to fetch: ${response.status}`, response.status)
  }
  return response
}

export async function fetchWithAuth(
  url: string,
  init?: RequestInit,
  extra?: {
    extraSuccessStatus: number[]
  },
): Promise<Response> {
  const token = useUserStore()?.token
  const requestConfig = init || {}
  requestConfig.headers = {
    ...requestConfig.headers,
    Authorization: `Bearer ${token}`,
  }

  return fetchWithError(url, requestConfig, extra)
}

const MAX_RETRIES = 3
const HTTP_STATUS_TO_NOT_RETRY = [400, 404]

export function doNotRetryPermanentErrors(failureCount: number, error: unknown): boolean {
  if (failureCount > MAX_RETRIES) {
    return false
  }

  if (
    typeof error === 'object' &&
    error !== null &&
    'status' in error &&
    HTTP_STATUS_TO_NOT_RETRY.includes(error.status as number)
  ) {
    console.log(`Aborting retry due to ${error.status} status`)
    return false
  }

  return true
}
