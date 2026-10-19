/** Server list: streamed page by page, region & ping on demand, player finder */

import { ClipboardCopy, Loader2, MapPin, Play, RefreshCw, Search, Square } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { api, on } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { Server } from '@/lib/types'

type Sort = 'playing' | 'ping' | null

export function ServersTab() {
  const t = useT()
  const notify = notifier(t)
  const { place, setJob, selected } = useStore()
  const [servers, setServers] = useState<Server[]>([])
  const [busy, setBusy] = useState(false)
  const [otherPlace, setOtherPlace] = useState('')
  const [player, setPlayer] = useState('')
  const [chosen, setChosen] = useState<string | null>(null)
  const [sort, setSort] = useState<Sort>(null)
  const generation = useRef(0)

  useEffect(() => {
    const stops = [
      on<{ generation: number; items: Server[]; done: boolean }>('servers', (e) => {
        if (e.generation !== generation.current) return
        setServers((s) => [...s, ...e.items])
        if (e.done) setBusy(false)
      }),
      on<{ id: string; region: string; ping?: number | null }>('server-region', (e) => setServers((s) => s.map((x) => (x.id === e.id ? { ...x, region: e.region, ping: e.ping ?? x.ping } : x)))),
      on<{ generation: number; server?: Server | null; error?: string }>('player-found', (e) => {
        if (e.generation !== generation.current) return
        setBusy(false)
        if (e.error) notify.error(e.error)
        else if (e.server) {
          setServers([e.server])
          notify.success('Found')
        } else notify.info('Not found')
      }),
    ]
    return () => stops.forEach((s) => s.then((stop) => stop()))
  }, [notify])

  const hasGame = /\d{3,}/.test(place) || /\d{3,}/.test(otherPlace)
  // false (and says why) when there's no game to list
  const gameChosen = () => {
    if (hasGame) return true
    notify.info('Choose a game first: its Place ID in the main window, or in Other Place ID here')
    return false
  }

  const refresh = async () => {
    if (busy) {
      await api.stopServers()
      return setBusy(false)
    }
    if (!gameChosen()) return
    setServers([])
    setBusy(true)
    try {
      generation.current = await api.listServers(place, otherPlace, selected[0] ?? null)
    } catch (e) {
      setBusy(false)
      notify.error(e)
    }
  }

  const find = async () => {
    if (!gameChosen()) return
    setServers([])
    setBusy(true)
    try {
      generation.current = await api.findPlayer(player, place, otherPlace)
    } catch (e) {
      setBusy(false)
      notify.error(e)
    }
  }

  const join = async (server: Server) => {
    if (!selected[0]) return notify.info('No Account Selected!')
    const result = await api.joinJob(selected[0], place, server.type === 'VIP' ? `VIP:${server.accessCode}` : server.id, otherPlace)
    if (result.result === 'error') notify.error(result.message)
  }

  const loadRegions = (from: Server) => {
    const index = servers.indexOf(from)
    const ids = servers.slice(index, index + 16).map((s) => s.id)
    setServers((s) => s.map((x) => (ids.includes(x.id) && !x.region ? { ...x, region: t('Loading') } : x)))
    api.loadRegions(selected[0] ?? null, place, ids).catch(notify.error)
  }

  const shown = useMemo(() => {
    if (!sort) return servers
    return [...servers].sort((a, b) => (sort === 'playing' ? b.playing - a.playing : (a.ping ?? 9999) - (b.ping ?? 9999)))
  }, [servers, sort])

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap gap-2">
        <Button onClick={refresh} variant={busy ? 'secondary' : 'default'}>
          {busy ? <Square /> : <RefreshCw />} {t(busy ? 'Cancel' : 'Refresh')}
        </Button>
        <Input className="w-auto min-w-44 flex-1 basis-52" value={otherPlace} onChange={(e) => setOtherPlace(e.target.value)} placeholder={t('Other Place ID (optional)')} />
        <form className="flex min-w-0 flex-1 basis-72 justify-end gap-2" onSubmit={(e) => { e.preventDefault(); find() }}>
          <Input className="min-w-48 flex-1" value={player} onChange={(e) => setPlayer(e.target.value)} placeholder={t('Find player by username')} />
          <Button type="submit" variant="secondary" disabled={!player.trim() || busy}><Search /> {t('Search')}</Button>
        </form>
      </div>

      <div className="min-h-0 flex-1 rounded-lg border">
        <ScrollArea className="h-full">
          <Table>
            <TableHeader className="sticky top-0 bg-card">
              <TableRow>
                <TableHead>{t('Job ID')}</TableHead>
                <TableHead className="cursor-pointer" onClick={() => setSort(sort === 'playing' ? null : 'playing')}>{t('Playing')} {sort === 'playing' && '↓'}</TableHead>
                <TableHead className="cursor-pointer" onClick={() => setSort(sort === 'ping' ? null : 'ping')}>{t('Ping')} {sort === 'ping' && '↑'}</TableHead>
                <TableHead>{t('Region')}</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {shown.map((s) => (
                <ContextMenu key={s.id}>
                  <ContextMenuTrigger asChild>
                    <TableRow data-state={chosen === s.id ? 'selected' : undefined} onClick={() => setChosen(s.id)} onDoubleClick={() => setJob(s.type === 'VIP' ? `VIP:${s.accessCode}` : s.id)}>
                      <TableCell className="font-mono text-xs">{s.type === 'VIP' && <span className="mr-2 rounded bg-primary/15 px-1.5 py-0.5 text-[10px] font-semibold text-primary">VIP</span>}{s.id}</TableCell>
                      <TableCell>{s.playing}/{s.maxPlayers}</TableCell>
                      <TableCell>{s.ping ?? ''}</TableCell>
                      <TableCell className="text-muted-foreground">{s.region}</TableCell>
                      <TableCell>
                        <Button size="icon-xs" variant="ghost" aria-label={t('Join Server')} onClick={(e) => { e.stopPropagation(); join(s) }}><Play /></Button>
                      </TableCell>
                    </TableRow>
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem onClick={() => join(s)}><Play /> {t('Join Server')}</ContextMenuItem>
                    <ContextMenuItem onClick={() => { navigator.clipboard.writeText(s.id); notify.success('Copied') }}><ClipboardCopy /> {t('Copy JobId')}</ContextMenuItem>
                    <ContextMenuItem onClick={() => loadRegions(s)}><MapPin /> {t('Load Region')}</ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              ))}
            </TableBody>
          </Table>
          {servers.length === 0 && (
            <div className="grid place-items-center py-16 text-sm text-muted-foreground">
              {busy ? <Loader2 className="animate-spin" /> : t(hasGame ? 'Press Refresh to list the servers of the game in the main window.' : 'Choose a game first: its Place ID in the main window, or in Other Place ID here')}
            </div>
          )}
        </ScrollArea>
      </div>
      <p className="text-xs text-muted-foreground">{t('Double click a server to put it in the Job ID box. Right click for more.')}</p>
    </div>
  )
}
