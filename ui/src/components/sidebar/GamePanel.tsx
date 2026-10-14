/** Game section: place (with recent games), server, shuffle, save to account and the big Join button */

import { History, Loader2, Play, Save, Shuffle, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Progress } from '@/components/ui/progress'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { GameIcon } from '@/components/common/GameTile'
import { useActions } from '@/hooks/useActions'
import { api, on } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'
import type { Game } from '@/lib/types'

/** A private server share link (roblox.com/share?code=…&type=Server): the game comes from Roblox when joining */
const isShareLink = (text: string) => /roblox\.com\/share(?:-links)?\/?\?\S*type=server/i.test(text) && /[?&]code=\w+/i.test(text)

/** Pasted links: a game page fills the place, a private server link goes to the server box */
function fromLink(text: string): { place?: string; job?: string } | null {
  if (isShareLink(text)) return { job: text.trim() }
  const game = /roblox\.com\/(?:[a-z]{2}\/)?games\/(\d+)/i.exec(text)
  if (!game) return null
  return text.includes('privateServerLinkCode') ? { place: game[1], job: text.trim() } : { place: game[1] }
}

export function GamePanel() {
  const t = useT()
  const actions = useActions()
  const { place, setPlace, job, setJob, selected, launching, setting, updateSetting } = useStore()
  const [info, setInfo] = useState<{ name: string; icon?: string } | null>(null)
  const [recent, setRecent] = useState<Game[]>([])
  const [shuffle, setShuffle] = useState(setting('General', 'ShuffleJobId') === 'true')
  const [joining, setJoining] = useState(false)
  // what the user typed or picked, saved as the place for the next start (an account's own saved place isn't)
  const typed = useRef<string | null>(null)
  const choose = (value: string) => {
    typed.current = value
    setPlace(value)
  }

  useEffect(() => {
    api.recentGames().then(setRecent)
    const stop = on<Game[]>('recent-games', setRecent)
    return () => void stop.then((s) => s())
  }, [])

  useEffect(() => void api.setShuffle(shuffle), [shuffle])

  // the game's name and icon, once the user stops typing (an answer for an older place is dropped)
  useEffect(() => {
    setInfo(null)
    if (!/\d{3,}/.test(place)) return
    let current = true
    const timer = setTimeout(() => api.placeInfo(place).then((i) => current && setInfo(i)), 500)
    return () => {
      current = false
      clearTimeout(timer)
    }
  }, [place])

  useEffect(() => {
    if (typed.current !== place) return
    const timer = setTimeout(() => {
      typed.current = null
      void updateSetting('General', 'SavedPlaceId', place.trim())
    }, 800)
    return () => clearTimeout(timer)
  }, [place, updateSetting])

  const paste = (e: React.ClipboardEvent) => {
    const link = fromLink(e.clipboardData.getData('text'))
    if (!link) return
    e.preventDefault()
    if (link.place) choose(link.place)
    if (link.job) setJob(link.job)
  }

  const join = async () => {
    setJoining(true)
    try {
      await actions.join()
    } finally {
      setJoining(false)
    }
    void updateSetting('General', 'SavedPlaceId', place)
  }

  const recentGame = recent.find((g) => String(g.placeId) === place)
  // a share link in the server box decides the game (Roblox tells which when joining)
  const shareLink = isShareLink(job)

  return (
    <section className="grid gap-3">
      <div className="flex items-center gap-3">
        <GameIcon src={shareLink ? undefined : (info?.icon ?? recentGame?.icon)} className="size-11" />
        <div className="min-w-0">
          <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{t('Game')}</div>
          <div className="truncate font-semibold">{shareLink ? t('Private server from a link') : place.trim() ? (info?.name ?? recentGame?.name ?? t('Current Place')) : t('No game chosen')}</div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <div className="grid gap-1.5">
          <div className="flex items-center justify-between">
            <Label htmlFor="place">{t('Place ID')}</Label>
            <Popover>
              <PopoverTrigger asChild>
                <Button variant="ghost" size="icon-xs" aria-label={t('Recent games')} disabled={recent.length === 0}>
                  <History />
                </Button>
              </PopoverTrigger>
              <PopoverContent align="start" className="w-72 p-1">
                <div className="px-2 py-1.5 text-xs font-medium text-muted-foreground">{t('Recent games')}</div>
                {recent.map((g) => (
                  <button key={g.placeId} className="flex w-full items-center gap-2 rounded-md p-1.5 text-left text-sm hover:bg-accent" onClick={() => choose(String(g.placeId))}>
                    <GameIcon src={g.icon} className="size-8" />
                    <span className="truncate">{g.name}</span>
                  </button>
                ))}
              </PopoverContent>
            </Popover>
          </div>
          <Input id="place" value={place} onChange={(e) => choose(e.target.value)} onPaste={paste} onFocus={(e) => e.target.select()} placeholder="xxxx" />
        </div>

        <div className="grid gap-1.5">
          <div className="flex items-center justify-between">
            <Label htmlFor="job">{t('Job ID')}</Label>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon-xs" className={shuffle ? 'text-emerald-500' : ''} aria-pressed={shuffle} aria-label={t('Shuffle JobId')} onClick={() => setShuffle(!shuffle)}>
                  <Shuffle />
                </Button>
              </TooltipTrigger>
              <TooltipContent className="max-w-60">{t(shuffle ? 'Shuffle is on: every join picks a random server' : 'Shuffle: pick a random server for every join')}</TooltipContent>
            </Tooltip>
          </div>
          <div className="flex gap-1">
            <Input id="job" value={job} onChange={(e) => setJob(e.target.value)} onPaste={paste} onFocus={(e) => e.target.select()} placeholder={t('Job ID or VIP link')} />
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="secondary" size="icon" disabled={selected.length === 0} onClick={actions.saveToAccount} aria-label={t('Save to account')}>
                  <Save />
                </Button>
              </TooltipTrigger>
              <TooltipContent className="max-w-64">{t('Save this Place ID and Job ID to the selected accounts, they join it every time')}</TooltipContent>
            </Tooltip>
          </div>
        </div>
      </div>

      {launching.total > 0 ? (
        <div className="grid gap-2 rounded-lg border p-3">
          <div className="flex items-center justify-between text-sm">
            <span>{t('Launching {0} of {1}', Math.min(launching.done + 1, launching.total), launching.total)}</span>
            <Button variant="ghost" size="xs" onClick={() => api.cancelLaunch()}>
              <X /> {t('Cancel')}
            </Button>
          </div>
          <Progress value={(launching.done / launching.total) * 100} className="h-1.5" />
        </div>
      ) : (
        <Button size="lg" className="h-11 text-base" disabled={selected.length === 0 || joining} onClick={join}>
          {joining ? <Loader2 className="animate-spin" /> : <Play />}
          {selected.length > 1 ? t('Join Server ({0})', selected.length) : t('Join Server')}
        </Button>
      )}
    </section>
  )
}
