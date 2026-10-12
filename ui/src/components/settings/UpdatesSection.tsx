/** Settings > General: the version, checking now, and how updates arrive */

import { RefreshCw, RotateCw } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { SettingRow, Toggle } from '@/components/common/SettingRow'
import { useUpdate } from '@/hooks/useUpdate'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import type { UpdateStatus } from '@/lib/types'

function describe(status: UpdateStatus, t: ReturnType<typeof useT>) {
  switch (status.state) {
    case 'checking':
      return t('Checking for updates…')
    case 'upToDate':
      return t('You have the latest version')
    case 'available':
      return t('Version {0} is available', status.version)
    case 'downloading':
      return `${t('Downloading version {0}', status.version)} ${Math.round(status.progress * 100)}%`
    case 'ready':
      return t('Version {0} is ready, it installs when the app restarts', status.version)
    case 'error':
      return t(status.message)
    case 'disabled':
      return t('Development build, it does not update itself')
    default:
      return ''
  }
}

export function UpdatesSection() {
  const t = useT()
  const notify = notifier(t)
  const { status, version } = useUpdate()
  const [busy, setBusy] = useState(false)
  const working = busy || status.state === 'checking' || status.state === 'downloading'

  const check = async () => {
    setBusy(true)
    try {
      await api.updateCheck(status.state === 'available')
    } catch (e) {
      notify.error(e)
    } finally {
      setBusy(false)
    }
  }

  return (
    <>
      <SettingRow title={t('Version {0}', version)} hint={describe(status, t)}>
        {status.state === 'ready' ? (
          <Button variant="secondary" onClick={() => api.updateRestart().catch(notify.error)}>
            <RotateCw /> {t('Restart to update')}
          </Button>
        ) : (
          <Button variant="secondary" disabled={working || status.state === 'disabled'} onClick={check}>
            <RefreshCw className={working ? 'animate-spin' : ''} /> {t(status.state === 'available' ? 'Download update' : 'Check now')}
          </Button>
        )}
      </SettingRow>
      <Toggle section="Updates" name="Check" title="Check for updates automatically" hint="When the app starts and every 6 hours, from the GitHub releases." />
      <Toggle section="Updates" name="AutoInstall" title="Download updates by themselves" hint="Installed the next time the app starts. Only updates signed by the developer are accepted." />
    </>
  )
}
