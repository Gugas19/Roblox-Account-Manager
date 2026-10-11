/** Live Anti-AFK status (the Rust engine sends it every second) */

import { useEffect, useState } from 'react'
import { api, on } from '@/lib/api'
import type { AntiAfkStatus } from '@/lib/types'

export function useAntiAfk() {
  const [status, setStatus] = useState<AntiAfkStatus | null>(null)

  useEffect(() => {
    api.antiAfkStatus().then(setStatus)
    const stop = on<AntiAfkStatus>('antiafk', setStatus)
    return () => void stop.then((s) => s())
  }, [])

  return status
}

export function clock(seconds: number) {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

export function duration(seconds: number) {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  return h > 0 ? `${h}h ${m}m` : `${m}m`
}
