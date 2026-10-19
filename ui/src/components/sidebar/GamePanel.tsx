/** Game section: place (with recent games), server, shuffle and the big Join button. What's typed is saved by itself,
 *  into the selected accounts (each has its own game) or, with "Same for all", once for every account */

import { ChevronDown, History, House, Loader2, Play, Shuffle, UserRoundSearch, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from '@/components/ui/input-group'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Progress } from '@/components/ui/progress'
import { Switch } from '@/components/ui/switch'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { GameIcon } from '@/components/common/GameTile'
import { useDialogs } from '@/components/dialogs/DialogHost'
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
  const { place, setPlace, job, setJob, selected, launching, setting, updateSetting, boot } = useStore()
  const dialogs = useDialogs()
  const [info, setInfo] = useState<{ name: string; icon?: string } | null>(null)
  const [recent, setRecent] = useState<Game[]>([])
  const [shuffle, setShuffle] = useState(setting('General', 'ShuffleJobId') === 'true')
  const [joining, setJoining] = useState(false)
  const sameForAll = setting('General', 'SameGameForAll') === 'true'
  // What's typed or picked is saved by itself: into the selected accounts (each remembers its own game), and as the
  // last game used (for the next start and for accounts without one). With "same game for all", only the latter.
  // The accounts are the ones selected while typing, even if the selection changes before the save
  const saveTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const remember = (nextPlace: string, nextJob: string) => {
    clearTimeout(saveTimer.current)
    const names = sameForAll ? [] : [...selected]
    saveTimer.current = setTimeout(() => {
      void updateSetting('General', 'SavedPlaceId', nextPlace.trim())
      void updateSetting('General', 'SavedJobId', nextJob.trim())
      if (names.length) api.savePlace(names, nextPlace.trim(), nextJob.trim()).catch(() => {})
    }, 800)
  }
  const choose = (value: string) => {
    setPlace(value)
    remember(value, job)
  }
  const chooseJob = (value: string) => {
    setJob(value)
    remember(place, value)
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

  const paste = (e: React.ClipboardEvent) => {
    const link = fromLink(e.clipboardData.getData('text'))
    if (!link) return
    e.preventDefault()
    const nextPlace = link.place ?? place
    const nextJob = link.job ?? job
    setPlace(nextPlace)
    setJob(nextJob)
    remember(nextPlace, nextJob)
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

  // following a player: from the Join menu, the username asked then
  const follow = async () => {
    const name = await dialogs.prompt({ title: 'Follow a player', label: 'Their username: the selected accounts join the server they are in.', defaultValue: setting('General', 'SavedFollowUser') || boot.followUser, confirm: 'Follow' })
    if (name?.trim()) await actions.follow(name.trim())
  }

  const recentGame = recent.find((g) => String(g.placeId) === place)
  // a share link in the server box decides the game (Roblox tells which when joining)
  const shareLink = isShareLink(job)

  return (
    <section className="grid gap-3">
      <div className="flex items-center gap-3">
        <GameIcon src={shareLink ? undefined : (info?.icon ?? recentGame?.icon)} className="size-11" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{t('Game')}</div>
            <Tooltip>
              <TooltipTrigger asChild>
                <label className="flex shrink-0 cursor-pointer items-center gap-1.5 text-xs text-muted-foreground">
                  <Switch size="sm" checked={sameForAll} onCheckedChange={(on) => void updateSetting('General', 'SameGameForAll', on ? 'true' : 'false')} />
                  {t('Same for all')}
                </label>
              </TooltipTrigger>
              <TooltipContent className="max-w-64">
                {t(sameForAll ? 'On: every account joins this game, choosing another account doesn\'t change it' : 'Off: each account remembers its own game, it shows when you choose the account')}
              </TooltipContent>
            </Tooltip>
          </div>
          <div className="truncate font-semibold">{shareLink ? t('Private server from a link') : place.trim() ? (info?.name ?? recentGame?.name ?? t('Current Place')) : t('No game chosen')}</div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <div className="grid gap-1">
          <Label htmlFor="place" className="text-xs text-muted-foreground">{t('Place ID')}</Label>
          <InputGroup>
            <InputGroupInput id="place" value={place} onChange={(e) => choose(e.target.value)} onPaste={paste} onFocus={(e) => e.target.select()} placeholder="xxxx" />
            {/* only with recent games: a disabled button would grey out the whole box */}
            {recent.length > 0 && (
            <InputGroupAddon align="inline-end">
            <Popover>
              <PopoverTrigger asChild>
                <InputGroupButton size="icon-xs" aria-label={t('Recent games')}>
                  <History />
                </InputGroupButton>
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
            </InputGroupAddon>
            )}
          </InputGroup>
        </div>

        <div className="grid gap-1">
          <Label htmlFor="job" className="text-xs text-muted-foreground">{t('Server (link or ID)')}</Label>
          <InputGroup>
            <InputGroupInput id="job" value={job} onChange={(e) => chooseJob(e.target.value)} onPaste={paste} onFocus={(e) => e.target.select()} placeholder={t('Job ID or VIP link')} />
            <InputGroupAddon align="inline-end">
              <Tooltip>
                <TooltipTrigger asChild>
                  <InputGroupButton size="icon-xs" className={shuffle ? 'text-emerald-500' : ''} aria-pressed={shuffle} aria-label={t('Shuffle JobId')} onClick={() => setShuffle(!shuffle)}>
                    <Shuffle />
                  </InputGroupButton>
                </TooltipTrigger>
                <TooltipContent className="max-w-60">{shuffle ? t('Shuffle is on: every join picks a random server') : t('Shuffle: pick a random server for every join')}</TooltipContent>
              </Tooltip>
            </InputGroupAddon>
          </InputGroup>
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
        <div className="flex">
          <Button size="lg" className="h-11 flex-1 rounded-r-none text-base" disabled={selected.length === 0 || joining} onClick={join}>
            {joining ? <Loader2 className="animate-spin" /> : <Play />}
            {selected.length > 1 ? t('Join Server ({0})', selected.length) : t('Join Server')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="lg" className="h-11 rounded-l-none border-l border-primary-foreground/20 px-3" disabled={selected.length === 0} aria-label={t('More ways to join')}>
                <ChevronDown />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56">
              <DropdownMenuItem onClick={follow}><UserRoundSearch /> {t('Follow a player…')}</DropdownMenuItem>
              <DropdownMenuItem onClick={actions.openHome}><House /> {t('Open Roblox home')}</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
    </section>
  )
}
