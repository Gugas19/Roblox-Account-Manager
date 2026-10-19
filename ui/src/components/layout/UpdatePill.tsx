/** Top bar (and the unlock screen): a new version, its download, then "restart to update" (hidden otherwise).
 *  The first time a new version shows up, a notification with a button says so too. */

import { ArrowUpCircle, RotateCw } from 'lucide-react'
import { useEffect } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { whatsNew } from '@/components/dialogs/WhatsNew'
import { useUpdate } from '@/hooks/useUpdate'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'

// each version is announced once (available, then ready), even when the pill moves from the unlock screen to the top bar
let announced: string | null = null

export function UpdatePill() {
  const t = useT()
  const notify = notifier(t)
  const { status, version } = useUpdate()

  // what's new in every version since this one first, then the download or the restart
  const download = async () => {
    if (await whatsNew('download')) await api.updateCheck(true).catch(notify.error)
  }
  const restart = async () => {
    if (await whatsNew('restart')) await api.updateRestart().catch(notify.error)
  }

  useEffect(() => {
    if (status.state !== 'available' && status.state !== 'ready') return
    const key = `${status.state}:${status.version}`
    if (announced === key) return
    announced = key

    if (status.state === 'ready') {
      toast(t('Version {0} is ready', status.version), {
        description: t('You have version {0}. Restart the app to update, or it updates the next time it starts.', version),
        action: { label: t('Restart'), onClick: () => void restart() },
        duration: 30000,
      })
    } else {
      toast(t('Version {0} is available', status.version), {
        description: t('You have version {0}, it is out of date.', version),
        action: { label: t('Download update'), onClick: () => void download() },
        duration: 30000,
      })
    }
  }, [status, version]) // eslint-disable-line react-hooks/exhaustive-deps

  if (status.state === 'available') {
    return (
      <Button variant="secondary" size="sm" className="gap-2 text-sky-500" onClick={download}>
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
      <Button variant="secondary" size="sm" className="gap-2 text-emerald-500" onClick={restart}>
        <RotateCw /> {t('Restart to update')}
      </Button>
    )
  }
  return null
}
