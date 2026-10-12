/** Games search, favorites and universes: all show game tiles that put their place in the main window */

import { ClipboardCopy, Loader2, Pencil, Play, Search, Star, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { GameTile } from '@/components/common/GameTile'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { Favorite, Game } from '@/lib/types'

function Grid({ games, loading, empty, onPick, extra }: { games: Game[]; loading: boolean; empty: string; onPick: (g: Game) => void; extra?: (g: Game) => React.ReactNode }) {
  const t = useT()
  if (loading) return <div className="grid place-items-center py-16"><Loader2 className="animate-spin text-muted-foreground" /></div>
  if (games.length === 0) return <p className="py-16 text-center text-sm text-muted-foreground">{t(empty)}</p>
  return (
    <ScrollArea className="h-full">
      <div className="grid grid-cols-2 gap-2 pr-3">
        {games.map((g, i) => (
          <GameTile key={`${g.placeId}-${i}`} game={g} onClick={() => onPick(g)}>
            {extra?.(g)}
          </GameTile>
        ))}
      </div>
    </ScrollArea>
  )
}

function usePick() {
  const t = useT()
  const notify = notifier(t)
  const { setPlace, setJob } = useStore()
  return (g: Game, job = '') => {
    setPlace(String(g.placeId))
    setJob(job)
    notify.success(t('{0} is now the game in the main window', g.name))
  }
}

export function GamesTab() {
  const t = useT()
  const notify = notifier(t)
  const pick = usePick()
  const [term, setTerm] = useState('')
  const [page, setPage] = useState(0)
  const [games, setGames] = useState<Game[]>([])
  const [loading, setLoading] = useState(false)

  const search = async (p = page) => {
    setLoading(true)
    try {
      setGames(await api.searchGames(term, p))
    } catch (e) {
      notify.error(e)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => void search(0), []) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); setPage(0); search(0) }}>
        <Input value={term} onChange={(e) => setTerm(e.target.value)} placeholder={t('Search games...')} />
        <Input type="number" min={0} className="w-20" value={page} onChange={(e) => setPage(Number(e.target.value))} aria-label={t('Page')} />
        <Button type="submit" disabled={loading}><Search /> {t('Search')}</Button>
      </form>
      <div className="min-h-0 flex-1">
        <Grid
          games={games}
          loading={loading}
          empty="No games found"
          onPick={(g) => pick(g)}
          extra={(g) => (
            <Button size="icon-sm" variant="ghost" aria-label={t('Add to Favorites')} onClick={async (e) => { e.stopPropagation(); try { await api.addFavorite(String(g.placeId), '', g.name); notify.success('Added to favorites') } catch (err) { notify.error(err) } }}>
              <Star />
            </Button>
          )}
        />
      </div>
    </div>
  )
}

export function FavoritesTab() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const pick = usePick()
  const { place, job, selected } = useStore()
  const [favorites, setFavorites] = useState<Favorite[]>([])
  const reload = () => api.favorites().then(setFavorites)

  useEffect(() => void reload(), [])

  const join = async (f: Favorite) => {
    if (!selected.length) return notify.info('No Account Selected!')
    try {
      const result = await api.join(selected, String(f.placeId), f.privateServer)
      if (result.result === 'error') notify.error(result.message)
    } catch (e) {
      notify.error(e)
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <Button className="self-start" disabled={!/\d{3,}/.test(place)} onClick={async () => { try { await api.addFavorite(place, job); await reload() } catch (e) { notify.error(e) } }}>
        <Star /> {t('Favorite the game in the main window')}
      </Button>
      <div className="min-h-0 flex-1">
        <Grid
          games={favorites}
          loading={false}
          empty="Your favorite games show here"
          onPick={(g) => pick(g, (g as Favorite).privateServer)}
          extra={(g) => {
            const f = g as Favorite
            return (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button size="icon-sm" variant="ghost" onClick={(e) => e.stopPropagation()} aria-label={t('More')}>⋯</Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent onClick={(e) => e.stopPropagation()}>
                  <DropdownMenuItem onClick={() => join(f)}><Play /> {t('Join')}</DropdownMenuItem>
                  <DropdownMenuItem onClick={async () => { const name = await dialogs.prompt({ title: 'Rename', label: 'Name', defaultValue: f.name }); if (name) { try { await api.renameFavorite(f.index, name); await reload() } catch (e) { notify.error(e) } } }}><Pencil /> {t('Rename')}</DropdownMenuItem>
                  <DropdownMenuItem onClick={() => { navigator.clipboard.writeText(String(f.placeId)); notify.success('Copied') }}><ClipboardCopy /> {t('Copy PlaceID')}</DropdownMenuItem>
                  <DropdownMenuItem className="text-destructive" onClick={async () => { if (await dialogs.confirm({ title: 'Remove Favorite', description: 'Are you sure?', destructive: true })) { try { await api.removeFavorite(f.index); await reload() } catch (e) { notify.error(e) } } }}><Trash2 /> {t('Remove')}</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            )
          }}
        />
      </div>
    </div>
  )
}

export function UniverseTab() {
  const t = useT()
  const notify = notifier(t)
  const pick = usePick()
  const { place } = useStore()
  const [universe, setUniverse] = useState('')
  const [games, setGames] = useState<Game[]>([])
  const [loading, setLoading] = useState(false)

  const view = async (id = universe) => {
    setLoading(true)
    try {
      setGames(await api.universePlaces(Number(id)))
    } catch (e) {
      notify.error(e)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex gap-2">
        <Button variant="secondary" disabled={loading || !/\d{3,}/.test(place)} onClick={async () => { try { const id = String(await api.universeId(place)); setUniverse(id); await view(id) } catch (e) { notify.error(e) } }}>
          {t('Get Universe ID')}
        </Button>
        <Input className="w-48" value={universe} onChange={(e) => setUniverse(e.target.value)} placeholder={t('Universe ID')} />
        <Button onClick={() => view()} disabled={!universe || loading}>{t('View Games')}</Button>
      </div>
      <div className="min-h-0 flex-1">
        <Grid games={games} loading={loading} empty="Every place of a game (universe) shows here" onPick={(g) => pick(g)} />
      </div>
    </div>
  )
}
