/** Groups page: the groups the chosen accounts are in, their role in each; leave one with several accounts at once,
 *  or join a group (through each account's browser, where Roblox may ask for a captcha) */

import { ExternalLink, Loader2, LogOut, UserPlus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { AccountPicker } from '@/components/common/AccountPicker'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { Page } from '@/components/layout/Page'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { GroupMembership, Owned } from '@/lib/types'

export function GroupsPage() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const { selected, accounts } = useStore()
  const [names, setNames] = useState<string[]>(selected.length ? selected : accounts.map((a) => a.username))
  const [data, setData] = useState<{ accounts: Owned<GroupMembership>[]; icons: Record<number, string> } | null>(null)
  const [busy, setBusy] = useState(false)

  const load = async () => {
    setBusy(true)
    try {
      const result = await api.groups(names)
      setData(result)
      for (const o of result.accounts.filter((o) => o.error)) notify.error(`${o.account}: ${o.error}`)
    } catch (e) {
      notify.error(e)
    } finally {
      setBusy(false)
    }
  }

  // each group once, with the accounts in it and their role
  const groups = useMemo(() => {
    const byId = new Map<number, { group: GroupMembership; members: [string, string][] }>()
    for (const o of data?.accounts ?? []) {
      for (const g of o.list) {
        const entry = byId.get(g.id) ?? { group: g, members: [] }
        entry.members.push([o.account, g.role])
        byId.set(g.id, entry)
      }
    }
    return [...byId.values()].sort((a, b) => b.members.length - a.members.length || a.group.name.localeCompare(b.group.name))
  }, [data])

  const leave = async (group: GroupMembership, members: string[]) => {
    if (!(await dialogs.confirm({ title: t('{0} leave {1}?', members.join(', '), group.name), destructive: true }))) return
    const results = await api.groupLeave(members, group.id).catch((e) => (notify.error(e), null))
    if (!results) return
    const failed = results.filter(([, error]) => error)
    for (const [account, error] of failed) notify.error(`${account}: ${error}`)
    if (results.length > failed.length) notify.success(t('{0} left {1}', results.length - failed.length, group.name))
    await load()
  }

  const join = async () => {
    const link = await dialogs.prompt({ title: 'Join a group', label: "The group's link or ID. Each account's browser opens on it and joins (Roblox may ask for a captcha there).", placeholder: 'https://www.roblox.com/communities/…' })
    if (!link?.trim()) return
    const url = /^\d+$/.test(link.trim()) ? `https://www.roblox.com/communities/${link.trim()}` : link.trim()
    await api.openBrowser(names, url, null, true).catch(notify.error)
  }

  return (
    <Page
      name="groups"
      title="Groups"
      description="The groups your accounts are in and their role in each. Leave a group with several accounts at once, or join one."
      actions={
        <div className="flex gap-2">
          <Button variant="secondary" disabled={!names.length} onClick={join}><UserPlus /> {t('Join a group')}</Button>
          <Button disabled={!names.length || busy} onClick={load}>{busy && <Loader2 className="animate-spin" />} {t('Load groups')}</Button>
        </div>
      }
    >
      <div className="mb-3">
        <AccountPicker value={names} onChange={setNames} />
      </div>
      <ScrollArea className="min-h-0 flex-1 pr-3">
        <div className="grid gap-2">
          {groups.map(({ group, members }) => (
            <div key={group.id} className="flex items-center gap-3 rounded-lg border p-3" data-testid="group">
              {data?.icons[group.id] ? <img src={data.icons[group.id]} alt="" className="size-11 shrink-0 rounded-md bg-muted" /> : <span className="size-11 shrink-0 rounded-md bg-muted" />}
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{group.name}</div>
                <div className="text-xs text-muted-foreground">{t('{0} members', group.members.toLocaleString())}</div>
                <div className="mt-1 flex flex-wrap gap-1">
                  {members.map(([account, role]) => (
                    <span key={account} className="rounded-full bg-muted px-2 py-0.5 text-[11px]">{account} · {role}</span>
                  ))}
                </div>
              </div>
              <Button variant="ghost" size="icon" aria-label={t('Open the group')} onClick={() => api.open('group', String(group.id))}><ExternalLink /></Button>
              <Button variant="ghost" size="sm" className="text-destructive" onClick={() => leave(group, members.map(([a]) => a))}><LogOut /> {t('Leave')}</Button>
            </div>
          ))}
          {data === null && !busy && <p className="py-12 text-center text-sm text-muted-foreground">{t('Choose accounts and load their groups.')}</p>}
          {data !== null && groups.length === 0 && <p className="py-12 text-center text-sm text-muted-foreground">{t('Not in any group')}</p>}
        </div>
      </ScrollArea>
    </Page>
  )
}
