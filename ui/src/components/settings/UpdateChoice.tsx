/** Asked once, on the first start: how updates arrive. The same choice stays in Settings > General */

import { BellRing, CloudOff, RefreshCw } from 'lucide-react'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { UPDATE_MODES, type UpdateMode } from '@/components/settings/UpdatesSection'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

const ICONS: Record<UpdateMode, React.ReactNode> = { notify: <BellRing />, auto: <RefreshCw />, off: <CloudOff /> }

export function UpdateChoice() {
  const t = useT()
  const { setting, updateSetting } = useStore()
  const open = setting('Updates', 'Chosen') !== 'true'

  const choose = async (mode: UpdateMode) => {
    await updateSetting('Updates', 'Check', String(UPDATE_MODES[mode].check))
    await updateSetting('Updates', 'AutoInstall', String(UPDATE_MODES[mode].install))
    await updateSetting('Updates', 'Chosen', 'true')
  }

  return (
    // a choice has to be made: no close button, Escape or click outside
    <Dialog open={open}>
      <DialogContent showCloseButton={false} onEscapeKeyDown={(e) => e.preventDefault()} onInteractOutside={(e) => e.preventDefault()} className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t('How should updates arrive?')}</DialogTitle>
          <DialogDescription>{t('Only versions signed by the developer are ever installed. You can change this later in Settings > General.')}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-2">
          {(['notify', 'auto', 'off'] as UpdateMode[]).map((mode) => (
            <button
              key={mode}
              onClick={() => void choose(mode)}
              className="flex items-start gap-3 rounded-xl border p-3.5 text-left transition-colors hover:border-primary hover:bg-primary/5"
            >
              <div className="mt-0.5 text-primary [&>svg]:size-4">{ICONS[mode]}</div>
              <div>
                <div className="flex items-center gap-2 text-sm font-medium">
                  {t(UPDATE_MODES[mode].label)}
                  {mode === 'notify' && <span className="rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary uppercase">{t('Recommended')}</span>}
                </div>
                <div className="mt-0.5 text-xs text-muted-foreground">{t(UPDATE_MODES[mode].hint)}</div>
              </div>
            </button>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  )
}
