/** The account list: search, groups, selection, drag and drop (accounts and cookie text), right click menu */

import { ChevronRight, Cookie, Globe, Search, UserRoundPlus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ContextMenu, ContextMenuTrigger } from '@/components/ui/context-menu'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { AccountMenu } from '@/components/accounts/AccountMenu'
import { AccountRow } from '@/components/accounts/AccountRow'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { Account } from '@/lib/types'

const COOKIE = /_\|WARNING:-DO-NOT-SHARE-THIS\./

function matches(a: Account, query: string) {
  const q = query.toLowerCase()
  return [a.username, a.alias, a.description, a.groupTitle].some((v) => v.toLowerCase().includes(q))
}

export function AccountList({ searchRef }: { searchRef: React.RefObject<HTMLInputElement | null> }) {
  const t = useT()
  const notify = notifier(t)
  const actions = useActions()
  const store = useStore()
  const { accounts, selected, select, hideUsernames, flag, setting, theme } = store
  const [query, setQuery] = useState('')
  const [collapsed, setCollapsed] = useState<string[]>([])
  const [dropOn, setDropOn] = useState<string | null>(null)
  const showGroups = setting('General', 'UIShowGroups') !== 'false'
  const showHeaders = (theme.ShowHeaders ?? 'True').toLowerCase() !== 'false'
  const showAging = !flag('General', 'DisableAgingAlert')
  const showPresence = flag('General', 'ShowPresence')

  const visible = useMemo(() => (query ? accounts.filter((a) => matches(a, query)) : accounts), [accounts, query])

  const groups = useMemo(() => {
    if (!showGroups) return [{ key: '', title: '', items: visible }]
    const map = new Map<string, Account[]>()
    for (const a of visible) map.set(a.group, [...(map.get(a.group) ?? []), a])
    return [...map.entries()]
      .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))
      .map(([key, items]) => ({ key, title: items[0].groupTitle, items }))
  }, [visible, showGroups])

  const dragged = (e: React.DragEvent, a: Account) => {
    const names = selected.includes(a.username) ? selected : [a.username]
    if (!selected.includes(a.username)) select(a.username)
    e.dataTransfer.setData('application/x-ram-accounts', JSON.stringify(names))
    e.dataTransfer.effectAllowed = 'move'
  }

  const dropped = async (e: React.DragEvent, before: string | null, group: string) => {
    e.preventDefault()
    setDropOn(null)
    const names = e.dataTransfer.getData('application/x-ram-accounts')
    if (names) return api.reorder(JSON.parse(names), before, group)

    const text = e.dataTransfer.getData('text/plain')
    if (COOKIE.test(text)) {
      const results = await api.importCookies(text)
      results.forEach((r) => (r.ok ? notify.success(t('Added {0}', r.text)) : notify.error(t('Failed to add {0}', r.text))))
    }
  }

  const over = (e: React.DragEvent, key: string) => {
    e.preventDefault()
    setDropOn(key)
  }

  if (accounts.length === 0) {
    return (
      <div className="grid h-full place-items-center p-8" onDragOver={(e) => e.preventDefault()} onDrop={(e) => dropped(e, null, 'Default')}>
        <div className="flex max-w-sm flex-col items-center gap-4 text-center">
          <div className="grid size-14 place-items-center rounded-2xl bg-primary/15 text-primary">
            <UserRoundPlus className="size-7" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">{t('No accounts yet')}</h2>
            <p className="mt-1 text-sm text-muted-foreground">{t('Log in to your Roblox accounts once, then join any game with any of them in one click.')}</p>
          </div>
          <div className="flex gap-2">
            <Button onClick={actions.addAccount}><Globe /> {t('Log in with the browser')}</Button>
            <Button variant="secondary" onClick={actions.importCookies}><Cookie /> {t('Cookie(s)')}</Button>
          </div>
          <p className="text-xs text-muted-foreground">{t('You can also drop cookies here')}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center gap-3 px-3 pt-3 pb-2">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input ref={searchRef} value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t('Search accounts...')} className="pl-8" onKeyDown={(e) => e.key === 'Escape' && setQuery('')} />
        </div>
        <span className="text-xs whitespace-nowrap text-muted-foreground">
          {query ? t('{0} of {1} accounts', visible.length, accounts.length) : t(accounts.length === 1 ? '{0} account' : '{0} accounts', accounts.length)}
          {selected.length > 1 && ` · ${t('{0} selected', selected.length)}`}
        </span>
      </div>

      {showHeaders && (
        <div className="grid grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)_minmax(0,1.6fr)] gap-3 border-b px-6 pb-2 text-xs font-medium text-muted-foreground">
          <span className="pl-6">{t('Username')}</span>
          <span>{t('Alias')}</span>
          <span>{t('Description')}</span>
        </div>
      )}

      <ContextMenu>
        <ContextMenuTrigger asChild>
          <ScrollArea className="min-h-0 flex-1">
            <div className="px-3 py-2" role="grid" aria-multiselectable onDragOver={(e) => e.preventDefault()} onDrop={(e) => dropped(e, null, 'Default')}>
              {visible.length === 0 && <p className="py-10 text-center text-sm text-muted-foreground">{t('No accounts match your search')}</p>}
              {groups.map((g) => {
                const closed = collapsed.includes(g.key)
                return (
                  <section key={g.key || 'all'} className="mb-1">
                    {showGroups && (
                      <button
                        className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs font-semibold tracking-wide text-primary uppercase hover:bg-accent ${dropOn === `group:${g.key}` ? 'bg-primary/10' : ''}`}
                        onClick={() => setCollapsed((c) => (closed ? c.filter((k) => k !== g.key) : [...c, g.key]))}
                        onDragOver={(e) => over(e, `group:${g.key}`)}
                        onDragLeave={() => setDropOn(null)}
                        onDrop={(e) => dropped(e, null, g.key)}
                      >
                        <ChevronRight className={`size-3.5 transition-transform ${closed ? '' : 'rotate-90'}`} />
                        {g.title}
                        <span className="font-normal text-muted-foreground normal-case">{g.items.length}</span>
                        <span className="ml-2 h-px flex-1 bg-border" />
                      </button>
                    )}
                    {!closed &&
                      g.items.map((a) => (
                        <AccountRow
                          key={a.username}
                          account={a}
                          selected={selected.includes(a.username)}
                          hideUsername={hideUsernames}
                          showAging={showAging}
                          showPresence={showPresence}
                          dropTarget={dropOn === a.username}
                          onSelect={(e) => select(a.username, e.shiftKey ? 'range' : e.ctrlKey || e.metaKey ? 'toggle' : 'only')}
                          onContext={() => !selected.includes(a.username) && select(a.username)}
                          onRename={(alias) => actions.setAlias(alias, [a.username])}
                          onDragStart={(e) => dragged(e, a)}
                          onDragOver={(e) => over(e, a.username)}
                          onDragLeave={() => setDropOn(null)}
                          onDrop={(e) => {
                            e.stopPropagation()
                            dropped(e, a.username, a.group)
                          }}
                          onJoin={actions.join}
                        />
                      ))}
                  </section>
                )
              })}
            </div>
          </ScrollArea>
        </ContextMenuTrigger>
        <AccountMenu showGroups={showGroups} toggleGroups={() => store.updateSetting('General', 'UIShowGroups', showGroups ? 'false' : 'true')} />
      </ContextMenu>
    </div>
  )
}
