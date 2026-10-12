/** Top bar: a new version, its download, then "restart to update" (hidden otherwise) */

import { ArrowUpCircle, RotateCw } from 'lucide-react'
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
  const { status } = useUpdate()

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
    const restart = async () => {
      const ok = await dialogs.confirm({ title: t('Restart to update to version {0}?', status.version), description: 'Anti-AFK stops while the app restarts.' })
      if (ok) await api.updateRestart().catch(notify.error)
    }
    return (
      <Button variant="secondary" size="sm" className="gap-2 text-emerald-500" onClick={restart}>
        <RotateCw /> {t('Restart to update')}
      </Button>
    )
  }
  return null
}
