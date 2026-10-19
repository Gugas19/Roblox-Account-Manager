/** History page: what each account did, newest first, by day; one account or all of them, exported as CSV */

import { ArrowRightLeft, CirclePlay, CircleStop, Download, LogIn, Power, PowerOff, RefreshCw, Trash2, Wrench, WifiOff, type LucideIcon } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { Page } from '@/components/layout/Page'
import { api, on } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import type { HistoryEvent } from '@/lib/types'

const KINDS: Record<string, { label: string; icon: LucideIcon; color: string }> = {
  joined: { label: 'Joined a game', icon: LogIn, color: 'text-emerald-500' },
  server: { label: 'Changed server', icon: ArrowRightLeft, color: 'text-sky-500' },
  disconnected: { label: 'Lost the connection', icon: WifiOff, color: 'text-destructive' },
  rejoined: { label: 'Rejoined', icon: RefreshCw, color: 'text-emerald-500' },
  closed: { label: 'Closed Roblox', icon: CircleStop, color: 'text-muted-foreground' },
  online: { label: 'Online', icon: Power, color: 'text-sky-500' },
  offline: { label: 'Offline', icon: PowerOff, color: 'text-muted-foreground' },
  playing: { label: 'In game', icon: CirclePlay, color: 'text-emerald-500' },
  studio: { label: 'In Studio', icon: Wrench, color: 'text-violet-500' },
}

const ALL = '*'

export function HistoryPage() {
  const { t, language } = useI18n()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const [events, setEvents] = useState<HistoryEvent[] | null>(null)
  const [account, setAccount] = useState(ALL)
  const [games, setGames] = useState<Record<number, string>>({})

  useEffect(() => {
    const load = () => api.historyList().then(setEvents, notify.error)
    load()
    const stop = on('history', load)
    return () => void stop.then((s) => s())
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const accounts = useMemo(() => [...new Set((events ?? []).map((e) => e.account))].sort((a, b) => a.localeCompare(b)), [events])
  const shown = useMemo(() => (events ?? []).filter((e) => account === ALL || e.account === account), [events, account])

  // names of the games the history doesn't have (from this PC's windows), asked once each
  useEffect(() => {
    for (const id of new Set(shown.slice(0, 200).filter((e) => e.placeId && !e.place).map((e) => e.placeId!))) {
      if (id in games) continue
      setGames((g) => ({ ...g, [id]: '' }))
      api.placeInfo(String(id)).then((info) => info && setGames((g) => ({ ...g, [id]: info.name })), () => {})
    }
  }, [shown, games])

  const days = useMemo(() => {
    const groups: [string, HistoryEvent[]][] = []
    for (const e of shown) {
      const day = new Date(e.at * 1000).toLocaleDateString(language, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })
      if (groups.at(-1)?.[0] === day) groups.at(-1)![1].push(e)
      else groups.push([day, [e]])
    }
    return groups
  }, [shown, language])

  const exportCsv = async () => {
    try {
      const path = await api.historyExport(account === ALL ? [] : [account])
      if (path) notify.success(t('Saved to {0}', path))
    } catch (e) {
      notify.error(e)
    }
  }

  const clear = async () => {
    if (!(await dialogs.confirm({ title: 'Clear the whole history?', description: 'Every event of every account is deleted.', destructive: true }))) return
    await api.historyClear()
    setEvents([])
  }

  return (
    <Page
      name="history"
      title="History"
      description="What your accounts did: games and servers joined on this PC, lost connections and rejoins, and when Roblox shows them online, in a game or in Studio."
      actions={
        <div className="flex flex-wrap justify-end gap-2">
          <Select value={account} onValueChange={setAccount}>
            <SelectTrigger className="w-48" aria-label={t('Account')}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>{t('All accounts')}</SelectItem>
              {accounts.map((a) => (
                <SelectItem key={a} value={a}>{a}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="secondary" disabled={!shown.length} onClick={exportCsv}><Download /> {t('Export CSV')}</Button>
          <Button variant="ghost" className="text-destructive" disabled={!events?.length} onClick={clear}><Trash2 /> {t('Clear')}</Button>
        </div>
      }
    >
      <ScrollArea className="min-h-0 flex-1 pr-3">
        <div className="mx-auto grid max-w-4xl gap-5">
          {days.map(([day, list]) => (
            <section key={day} className="grid gap-1">
              <h2 className="sticky top-0 bg-card py-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">{day}</h2>
              {list.map((e, i) => {
                const { label, icon: Icon, color } = KINDS[e.kind] ?? { label: e.kind, icon: CirclePlay, color: '' }
                const game = e.place || (e.placeId ? games[e.placeId] : '') || (e.placeId ? t('Place {0}', e.placeId) : '')
                return (
                  <div key={`${e.at}-${i}`} className="flex items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-accent" data-testid="history-event">
                    <span className="w-16 shrink-0 text-xs whitespace-nowrap text-muted-foreground tabular-nums">{new Date(e.at * 1000).toLocaleTimeString(language, { hour: '2-digit', minute: '2-digit' })}</span>
                    <Icon className={`size-4 shrink-0 ${color}`} />
                    <span className="w-40 shrink-0 truncate font-medium">{e.account}</span>
                    <span className="shrink-0">{t(label)}</span>
                    {game && <span className="min-w-0 truncate text-muted-foreground">· {game}</span>}
                    {e.jobId && <span className="ml-auto shrink-0 font-mono text-xs text-muted-foreground" title={e.jobId}>{e.jobId.slice(0, 8)}…</span>}
                  </div>
                )
              })}
            </section>
          ))}
          {events !== null && !shown.length && <p className="py-12 text-center text-sm text-muted-foreground">{t('Nothing yet. Join games with your accounts and what they do shows here.')}</p>}
        </div>
      </ScrollArea>
    </Page>
  )
}
