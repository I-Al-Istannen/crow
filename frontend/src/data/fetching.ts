import { useUserStore } from '@/stores/user.ts'

export const BACKEND_URL: string = import.meta.env.VITE_BACKEND_URL

export class FetchError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(`(HTTP ${status.toString()}) ${message}`)
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
  if (!response.ok && !extra?.extraSuccessStatus.includes(response.status)) {
    // TODO: Prettify WebErrors
    let text = await response.text().catch(() => 'unknown')
    try {
      const json = JSON.parse(text)
      text = json.error
    } catch {}
    throw new FetchError(text, response.status)
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
  const token = useUserStore().token
  const requestConfig = init ?? {}
  requestConfig.headers = {
    ...(requestConfig.headers as Record<string, string>),
  }
  if (token) {
    requestConfig.headers.Authorization = `Bearer ${token}`
  }

  return fetchWithError(url, requestConfig, extra)
}
