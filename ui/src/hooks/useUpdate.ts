/** The auto updater's state, live (Rust sends it on every change) */

import { useEffect, useState } from 'react'
import { api, on } from '@/lib/api'
import type { UpdateStatus } from '@/lib/types'

export function useUpdate() {
  const [status, setStatus] = useState<UpdateStatus>({ state: 'idle' })
  const [version, setVersion] = useState('')

  useEffect(() => {
    api.updateStatus().then((r) => {
      setStatus(r.status)
      setVersion(r.version)
    })
    const stop = on<UpdateStatus>('update', setStatus)
    return () => void stop.then((s) => s())
  }, [])

  return { status, version }
}
