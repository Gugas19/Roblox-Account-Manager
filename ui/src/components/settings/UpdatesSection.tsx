/** Settings > General: the version, checking now, and how updates arrive */

import { RefreshCw, RotateCw } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { SettingRow } from '@/components/common/SettingRow'
import { whatsNew } from '@/components/dialogs/WhatsNew'
import { useUpdate } from '@/hooks/useUpdate'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
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

/** One choice instead of two switches, so turning updates off is obvious */
export type UpdateMode = 'auto' | 'notify' | 'off'

export const UPDATE_MODES: Record<UpdateMode, { label: string; hint: string; check: boolean; install: boolean }> = {
  auto: { label: 'Automatic', hint: 'New versions download by themselves and install the next time the app starts. Only updates signed by the developer are accepted.', check: true, install: true },
  notify: { label: 'Only tell me', hint: 'The app tells you about a new version, it downloads only when you click the button.', check: true, install: false },
  off: { label: 'Off', hint: 'The app never contacts GitHub by itself. Use "Check now" whenever you want to look for a new version.', check: false, install: false },
}

export function UpdatesSection() {
  const t = useT()
  const notify = notifier(t)
  const { status, version } = useUpdate()
  const { setting, updateSetting } = useStore()
  const mode: UpdateMode = setting('Updates', 'Check') === 'false' ? 'off' : setting('Updates', 'AutoInstall') === 'false' ? 'notify' : 'auto'

  const changeMode = async (m: UpdateMode) => {
    try {
      await updateSetting('Updates', 'Check', String(UPDATE_MODES[m].check))
      await updateSetting('Updates', 'AutoInstall', String(UPDATE_MODES[m].install))
    } catch (e) {
      notify.error(e)
    }
  }
  const [busy, setBusy] = useState(false)
  const working = busy || status.state === 'checking' || status.state === 'downloading'

  const check = async () => {
    // a download shows what's new first
    if (status.state === 'available' && !(await whatsNew('download'))) return
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
          <Button variant="secondary" onClick={async () => (await whatsNew('restart')) && api.updateRestart().catch(notify.error)}>
            <RotateCw /> {t('Restart to update')}
          </Button>
        ) : (
          <Button variant="secondary" disabled={working || status.state === 'disabled'} onClick={check}>
            <RefreshCw className={working ? 'animate-spin' : ''} /> {t(status.state === 'available' ? 'Download update' : 'Check now')}
          </Button>
        )}
      </SettingRow>
      <SettingRow title={t('Automatic updates')} hint={UPDATE_MODES[mode].hint}>
        <Select value={mode} onValueChange={(m) => changeMode(m as UpdateMode)}>
          <SelectTrigger className="w-52">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {(Object.keys(UPDATE_MODES) as UpdateMode[]).map((m) => (
              <SelectItem key={m} value={m}>{t(UPDATE_MODES[m].label)}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </SettingRow>
    </>
  )
}
