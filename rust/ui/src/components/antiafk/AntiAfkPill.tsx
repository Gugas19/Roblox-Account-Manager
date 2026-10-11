/** Top bar button: Anti-AFK on/off at a glance, with the countdown */

import { ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { clock, useAntiAfk } from '@/hooks/useAntiAfk'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function AntiAfkPill() {
  const t = useT()
  const status = useAntiAfk()
  const { setPanel } = useStore()
  const running = status?.running

  return (
    <Button variant={running ? 'secondary' : 'ghost'} size="sm" className={running ? 'gap-2 text-emerald-500' : 'gap-2 text-muted-foreground'} onClick={() => setPanel('antiafk')}>
      <span className={`size-2 rounded-full ${running ? 'animate-pulse bg-emerald-500' : 'bg-muted-foreground/50'}`} />
      <ShieldCheck />
      {t('Anti-AFK')}
      {running && status?.secondsToNext != null && <span className="tabular-nums">{clock(status.secondsToNext)}</span>}
    </Button>
  )
}
