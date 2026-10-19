/** Friends page: one account's friends and where they are (join them), its friend requests, and who can see it online
 *  or join it */

import { Check, Copy, ExternalLink, LogIn, MoreHorizontal, RefreshCw, Search, UserMinus, X } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { Page } from '@/components/layout/Page'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { Friend, Visibility } from '@/lib/types'

const STATUS = [
  { label: 'Offline', dot: 'bg-muted-foreground/40' },
  { label: 'Online', dot: 'bg-sky-500' },
  { label: 'In game', dot: 'bg-emerald-500' },
  { label: 'In Studio', dot: 'bg-violet-500' },
]

const FILTERS = [
  { value: 'all', label: 'All' },
  { value: '2', label: 'In game' },
  { value: '3', label: 'In Studio' },
  { value: '1', label: 'Online' },
  { value: '0', label: 'Offline' },
]

/** Roblox's visibility values, in words (pairs, so tools/strings.py finds the texts) */
const AUDIENCE: Record<string, string> = Object.fromEntries([
  ['AllUsers', 'Everyone'],
  ['Everyone', 'Everyone'],
  ['FriendsFollowingAndFollowers', 'Friends, followers and following'],
  ['Followers', 'Friends, followers and following'],
  ['FriendsAndFollowing', 'Friends and people I follow'],
  ['Following', 'Friends and people I follow'],
  ['Friends', 'Friends'],
  ['TrustedFriends', 'Trusted friends'],
  ['NoOne', 'No one'],
])

const SETTING_TITLES: Record<string, string> = Object.fromEntries([
  ['whoCanJoinMeInExperiences', 'Who can join me in games'],
  ['whoCanSeeMyOnlineStatus', 'Who can see me online'],
])

function Avatar({ src }: { src?: string | null }) {
  return src ? <img src={src} alt="" className="size-9 shrink-0 rounded-full bg-muted" /> : <span className="size-9 shrink-0 rounded-full bg-muted" />
}

export function FriendsPage() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const { accounts, selected } = useStore()
  const [name, setName] = useState(selected[0] ?? accounts[0]?.username ?? '')
  const [friends, setFriends] = useState<Friend[] | null>(null)
  const [requests, setRequests] = useState<Friend[] | null>(null)
  const [visibility, setVisibility] = useState<Visibility[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState('all')

  const load = async (account = name) => {
    if (!account) return
    setBusy(true)
    setFriends(null)
    setRequests(null)
    setVisibility(null)
    await Promise.all([
      api.friendsList(account).then(setFriends, (e) => { setFriends([]); notify.error(e) }),
      api.friendRequests(account).then(setRequests, () => setRequests([])),
      api.visibilityGet(account).then(setVisibility, () => setVisibility([])),
    ])
    setBusy(false)
  }

  useEffect(() => void load(), [name]) // eslint-disable-line react-hooks/exhaustive-deps

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase()
    return (friends ?? []).filter((f) => (filter === 'all' || String(f.kind) === filter) && (!q || f.name.toLowerCase().includes(q) || f.displayName.toLowerCase().includes(q)))
  }, [friends, query, filter])
  const count = (kind: string) => (friends ?? []).filter((f) => kind === 'all' || String(f.kind) === kind).length

  const join = async (f: Friend) => {
    try {
      const result = await api.follow([name], f.id)
      if (result.result === 'error') notify.error(result.message)
    } catch (e) {
      notify.error(e)
    }
  }

  const act = async (f: Friend, action: 'accept' | 'decline' | 'remove') => {
    if (action === 'remove' && !(await dialogs.confirm({ title: t('Remove {0} from the friends of {1}?', f.name, name), destructive: true }))) return
    try {
      await api.friendAct(name, f.id, action)
      notify.success(action === 'accept' ? 'Accepted' : action === 'decline' ? 'Declined' : 'Removed')
      if (action === 'remove') setFriends((list) => list?.filter((x) => x.id !== f.id) ?? null)
      else {
        setRequests((list) => list?.filter((x) => x.id !== f.id) ?? null)
        if (action === 'accept') void api.friendsList(name).then(setFriends, () => {})
      }
    } catch (e) {
      notify.error(e)
    }
  }

  const changeVisibility = async (field: string, value: string) => {
    try {
      const [[, error]] = await api.visibilitySet([name], field, value)
      if (error) notify.error(error)
      else notify.success('Saved')
      setVisibility(await api.visibilityGet(name))
    } catch (e) {
      notify.error(e)
    }
  }

  return (
    <Page
      name="friends"
      title="Friends"
      description="An account's friends and where they are, its friend requests, and who can see it online or join it."
      actions={
        <div className="flex gap-2">
          <Select value={name} onValueChange={setName}>
            <SelectTrigger className="w-52" aria-label={t('Account')}>
              <SelectValue placeholder={t('Choose an account')} />
            </SelectTrigger>
            <SelectContent>
              {accounts.map((a) => (
                <SelectItem key={a.username} value={a.username}>{a.username}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="secondary" size="icon" aria-label={t('Refresh')} disabled={busy || !name} onClick={() => load()}>
            <RefreshCw className={busy ? 'animate-spin' : ''} />
          </Button>
        </div>
      }
    >
      {visibility && visibility.length > 0 && (
        <div className="mb-3 flex flex-wrap gap-x-6 gap-y-2 rounded-lg border p-3" data-testid="visibility">
          {visibility.map((v) => (
            <label key={v.field} className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{t(SETTING_TITLES[v.field] ?? v.field)}</span>
              <Select value={v.current} onValueChange={(value) => changeVisibility(v.field, value)}>
                <SelectTrigger className="h-8 w-auto min-w-40">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {[...new Set([v.current, ...v.options])].map((o) => (
                    <SelectItem key={o} value={o} disabled={!v.options.includes(o)}>{t(AUDIENCE[o] ?? o)}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </label>
          ))}
        </div>
      )}

      <Tabs defaultValue="friends" className="flex min-h-0 flex-1 flex-col">
        <div className="flex flex-wrap items-center gap-2">
          <TabsList>
            <TabsTrigger value="friends">{t('Friends')} {friends ? `(${friends.length})` : ''}</TabsTrigger>
            <TabsTrigger value="requests">{t('Requests')} {requests?.length ? `(${requests.length})` : ''}</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="friends" className="mt-3 flex min-h-0 flex-1 flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <div className="relative min-w-48 flex-1">
              <Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input className="pl-8" value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t('Search friends…')} />
            </div>
            <div className="flex flex-wrap gap-1">
              {FILTERS.map((f) => (
                <Button key={f.value} variant={filter === f.value ? 'secondary' : 'ghost'} size="sm" aria-pressed={filter === f.value} onClick={() => setFilter(f.value)}>
                  {t(f.label)} <span className="text-muted-foreground tabular-nums">{count(f.value)}</span>
                </Button>
              ))}
            </div>
          </div>
          <ScrollArea className="min-h-0 flex-1 pr-3">
            <div className="grid gap-1">
              {shown.map((f) => {
                const status = STATUS[f.kind] ?? STATUS[0]
                return (
                  <div key={f.id} className="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-accent" data-testid="friend">
                    <Avatar src={f.avatar} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-medium">
                        {f.displayName || f.name} <span className="font-normal text-muted-foreground">@{f.name}</span>
                      </div>
                      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                        <span className={`size-2 shrink-0 rounded-full ${status.dot}`} />
                        <span className="truncate">{f.location && f.kind >= 2 ? `${t(status.label)}: ${f.location}` : t(status.label)}</span>
                      </div>
                    </div>
                    {f.kind === 2 && (
                      <Button variant="secondary" size="sm" onClick={() => join(f)}>
                        <LogIn /> {t('Join')}
                      </Button>
                    )}
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon" aria-label={t('More')}>
                          <MoreHorizontal />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => api.open('profile', String(f.id))}><ExternalLink /> {t('Open profile')}</DropdownMenuItem>
                        <DropdownMenuItem onClick={() => navigator.clipboard.writeText(f.name).then(() => notify.success('Copied'))}><Copy /> {t('Copy Username')}</DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem className="text-destructive focus:text-destructive" onClick={() => act(f, 'remove')}><UserMinus /> {t('Remove friend')}</DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                )
              })}
              {friends === null && <p className="py-12 text-center text-sm text-muted-foreground">{t('Loading…')}</p>}
              {friends !== null && shown.length === 0 && <p className="py-12 text-center text-sm text-muted-foreground">{t(friends.length ? 'No friends match' : 'No friends yet')}</p>}
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="requests" className="mt-3 min-h-0 flex-1">
          <ScrollArea className="h-full pr-3">
            <div className="grid gap-1">
              {(requests ?? []).map((f) => (
                <div key={f.id} className="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-accent" data-testid="friend-request">
                  <Avatar src={f.avatar} />
                  <div className="min-w-0 flex-1 truncate text-sm font-medium">
                    {f.displayName || f.name} <span className="font-normal text-muted-foreground">@{f.name}</span>
                  </div>
                  <Button variant="secondary" size="sm" onClick={() => act(f, 'accept')}><Check /> {t('Accept')}</Button>
                  <Button variant="ghost" size="sm" onClick={() => act(f, 'decline')}><X /> {t('Decline')}</Button>
                </div>
              ))}
              {requests !== null && requests.length === 0 && <p className="py-12 text-center text-sm text-muted-foreground">{t('No friend requests')}</p>}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </Page>
  )
}
