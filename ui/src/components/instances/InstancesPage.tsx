/** Instances page: every Roblox open on this PC, with its account, game and server; bring one to the front, close it,
 *  send the selected accounts to its server, choose which ones Anti-AFK keeps active, and the window tools */

import { AppWindow, Copy, Eye, EyeOff, LayoutGrid, Lock, UsersRound, Volume2, VolumeX, X } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { GameIcon } from '@/components/common/GameTile'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { Page } from '@/components/layout/Page'
import { duration, useAntiAfk } from '@/hooks/useAntiAfk'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { AntiAfkWindow, JoinResult } from '@/lib/types'

function IconAction({ label, onClick, disabled, destructive, children }: { label: string; onClick: () => void; disabled?: boolean; destructive?: boolean; children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={label} disabled={disabled} className={destructive ? 'text-destructive' : ''} onClick={onClick}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

export function InstancesPage() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const status = useAntiAfk()
  const { selected, setting, updateSetting } = useStore()
  const windows = status?.windows ?? []
  const included = setting('AntiAFK', 'Accounts').split(',').map((s) => s.trim()).filter(Boolean)
  // game names and icons, asked once per game
  const [games, setGames] = useState<Record<number, { name: string; icon?: string } | null>>({})

  useEffect(() => {
    for (const id of new Set(windows.map((w) => w.placeId).filter((p): p is number => !!p))) {
      if (id in games) continue
      setGames((g) => ({ ...g, [id]: null }))
      api.placeInfo(String(id)).then((info) => info && setGames((g) => ({ ...g, [id]: info })), () => {})
    }
  }, [windows, games])

  const tool = async (kind: Parameters<typeof api.antiAfkTool>[0]) => {
    try {
      notify.success(await api.antiAfkTool(kind))
    } catch (e) {
      notify.error(e)
    }
  }

  const toggleAccount = (name: string, on: boolean) => {
    // an empty list means every account, the first unticked one starts the list with all the others
    const all = windows.map((w) => w.account).filter((a): a is string => !!a)
    const base = included.length ? included : all
    const next = on ? [...new Set([...base, name])] : base.filter((a) => a !== name)
    void updateSetting('AntiAFK', 'Accounts', next.length === all.length ? '' : next.join(','))
  }

  const sendHere = async (w: AntiAfkWindow) => {
    try {
      const result: JoinResult = await api.instanceJoin(w.pid, selected)
      if (result.result === 'error') notify.error(result.message)
    } catch (e) {
      notify.error(e)
    }
  }

  const close = async (w: AntiAfkWindow) => {
    if (!(await dialogs.confirm({ title: t('Close the Roblox of {0}?', w.account ?? `PID ${w.pid}`), destructive: true }))) return
    await api.instanceClose(w.pid).catch(notify.error)
  }

  const noWindows = windows.length === 0
  const others = (w: AntiAfkWindow) => selected.filter((n) => n !== w.account).length

  return (
    <Page
      name="instances"
      title="Instances"
      description="Every Roblox open on this PC: its account, game and server. The ticked ones are kept active by Anti-AFK."
      actions={
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('grid')}><LayoutGrid /> {t('Arrange in a grid')}</Button>
          {status?.hidden ? (
            <Button variant="secondary" size="sm" onClick={() => tool('show')}><Eye /> {t('Show windows')}</Button>
          ) : (
            <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('hide')}><EyeOff /> {t('Hide windows')}</Button>
          )}
          <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('mute')}><VolumeX /> {t('Mute all')}</Button>
          <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('unmute')}><Volume2 /> {t('Unmute all')}</Button>
        </div>
      }
    >
      <div className="mb-3 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
        <span className="flex-1">{windows.length === 1 ? t('1 Roblox window open') : t('{0} Roblox windows open', windows.length)}</span>
        <Button variant="ghost" size="sm" disabled={noWindows} onClick={() => tool('seeThrough')}>{t('See-through')}</Button>
        <Button variant="ghost" size="sm" disabled={noWindows} onClick={() => tool('solid')}>{t('Solid')}</Button>
        <Button variant="ghost" size="sm" className="text-destructive" disabled={noWindows} onClick={async () => { if (await dialogs.confirm({ title: 'Close every Roblox window?', destructive: true })) await tool('closeAll') }}>
          <X /> {t('Close all')}
        </Button>
      </div>

      <ScrollArea className="min-h-0 flex-1 pr-3">
        <div className="grid gap-2">
          {windows.map((w) => {
            const game = w.placeId ? games[w.placeId] : null
            return (
              <div key={w.pid} className="flex items-center gap-3 rounded-lg border p-3" data-testid="instance">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span>
                      <Checkbox aria-label={t('Anti-AFK')} checked={w.included} disabled={!w.account} onCheckedChange={(on) => w.account && toggleAccount(w.account, on === true)} />
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>{t(w.account ? 'Kept active by Anti-AFK' : 'Anti-AFK only knows the accounts in the list')}</TooltipContent>
                </Tooltip>
                <GameIcon src={game?.icon} className="size-10" />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate font-medium">{w.account ?? t('Roblox (not opened by the app)')}</span>
                    <Badge variant={w.state === 'disconnected' ? 'destructive' : 'secondary'}>{w.state === 'playing' ? t('In game') : w.state === 'disconnected' ? t('Disconnected') : t('Loading')}</Badge>
                    {w.private && <Badge variant="outline" className="gap-1"><Lock className="size-3" /> {t('Private server')}</Badge>}
                  </div>
                  <div className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                    <span className="truncate">{game?.name ?? (w.placeId ? t('Place {0}', w.placeId) : t('Not in a game yet'))}</span>
                    {w.jobId && (
                      <button className="flex shrink-0 cursor-pointer items-center gap-1 font-mono hover:text-foreground" title={w.jobId} onClick={() => navigator.clipboard.writeText(w.jobId!).then(() => notify.success('Copied'))}>
                        · {w.jobId.slice(0, 8)}… <Copy className="size-3" />
                      </button>
                    )}
                    <span className="shrink-0">· {duration(w.secondsRunning)} · PID {w.pid}</span>
                  </div>
                </div>
                <div className="flex shrink-0 items-center">
                  <IconAction label={t('Bring to front')} onClick={() => api.instanceFocus(w.pid).catch(notify.error)}>
                    <AppWindow />
                  </IconAction>
                  <IconAction label={others(w) ? t('Send the selected accounts here ({0})', others(w)) : t('Select accounts in the list to send them to this server')} disabled={!w.placeId || others(w) === 0} onClick={() => sendHere(w)}>
                    <UsersRound />
                  </IconAction>
                  <IconAction label={t('Close')} destructive onClick={() => close(w)}>
                    <X />
                  </IconAction>
                </div>
              </div>
            )
          })}
          {noWindows && <p className="py-12 text-center text-sm text-muted-foreground">{t('No Roblox windows open. Join a game and they show here.')}</p>}
        </div>
      </ScrollArea>
    </Page>
  )
}
