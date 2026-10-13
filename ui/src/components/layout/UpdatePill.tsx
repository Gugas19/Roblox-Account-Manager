/** Top bar: a new version, its download, then "restart to update" (hidden otherwise).
 *  The first time a new version shows up, a notification with a button says so too. */

import { ArrowUpCircle, RotateCw } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { useUpdate } from '@/hooks/useUpdate'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'

export function UpdatePill() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const { status, version } = useUpdate()
  // each version is announced once (available, then ready)
  const announced = useRef<string | null>(null)

  const restart = async (newVersion: string) => {
    const ok = await dialogs.confirm({ title: t('Restart to update to version {0}?', newVersion), description: 'Anti-AFK stops while the app restarts.' })
    if (ok) await api.updateRestart().catch(notify.error)
  }

  useEffect(() => {
    if (status.state !== 'available' && status.state !== 'ready') return
    const key = `${status.state}:${status.version}`
    if (announced.current === key) return
    announced.current = key

    if (status.state === 'ready') {
      toast(t('Version {0} is ready', status.version), {
        description: t('You have version {0}. Restart the app to update, or it updates the next time it starts.', version),
        action: { label: t('Restart'), onClick: () => void restart(status.version) },
        duration: 30000,
      })
    } else {
      toast(t('Version {0} is available', status.version), {
        description: t('You have version {0}, it is out of date.', version),
        action: { label: t('Download update'), onClick: () => void api.updateCheck(true).catch(notify.error) },
        duration: 30000,
      })
    }
  }, [status, version]) // eslint-disable-line react-hooks/exhaustive-deps

  if (status.state === 'available') {
    return (
      <Button variant="secondary" size="sm" className="gap-2 text-sky-500" onClick={() => api.updateCheck(true).catch(notify.error)}>
        <ArrowUpCircle /> {t('Update {0}', status.version)}
      </Button>
    )
  }
  if (status.state === 'downloading') {
    return (
      <div className="mr-2 flex w-36 flex-col gap-1 text-xs text-muted-foreground">
        {t('Downloading version {0}', status.version)}
        <Progress value={status.progress * 100} className="h-1.5" />
      </div>
    )
  }
  if (status.state === 'ready') {
    return (
      <Button variant="secondary" size="sm" className="gap-2 text-emerald-500" onClick={() => restart(status.version)}>
        <RotateCw /> {t('Restart to update')}
      </Button>
    )
  }
  return null
}
