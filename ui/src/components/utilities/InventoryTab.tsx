/** Avatar > Inventory: what the chosen accounts own, all of it, what they all have, or what only one has */

import { Loader2, Search } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { AccountPicker } from '@/components/common/AccountPicker'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { InventoryItem, Owned } from '@/lib/types'

const MODES = [
  { value: 'all', label: 'All items' },
  { value: 'shared', label: 'Shared by all' },
  { value: 'unique', label: 'Only one has it' },
]

/** Roblox's item kinds, in words (pairs, so tools/strings.py finds the texts) */
const KINDS: [string, string][] = [
  ['Hat', 'Hats'],
  ['HairAccessory', 'Hair'],
  ['FaceAccessory', 'Face accessories'],
  ['NeckAccessory', 'Neck accessories'],
  ['ShoulderAccessory', 'Shoulder accessories'],
  ['FrontAccessory', 'Front accessories'],
  ['BackAccessory', 'Back accessories'],
  ['WaistAccessory', 'Waist accessories'],
  ['Shirt', 'Shirts'],
  ['Pants', 'Pants'],
  ['TShirt', 'T-Shirts'],
  ['Gear', 'Gear'],
  ['EmoteAnimation', 'Emotes'],
]

export function InventoryTab() {
  const t = useT()
  const notify = notifier(t)
  const { selected, accounts } = useStore()
  const [names, setNames] = useState<string[]>(selected.length ? selected : accounts.slice(0, 1).map((a) => a.username))
  const [owned, setOwned] = useState<Owned<InventoryItem>[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [mode, setMode] = useState('all')
  const [kind, setKind] = useState('*')
  const [query, setQuery] = useState('')
  const [pictures, setPictures] = useState<Record<number, string>>({})

  const load = async () => {
    setBusy(true)
    try {
      const result = await api.inventories(names)
      setOwned(result)
      for (const o of result.filter((o) => o.error)) notify.error(`${o.account}: ${o.error}`)
    } catch (e) {
      notify.error(e)
    } finally {
      setBusy(false)
    }
  }

  // each item once, with who owns it
  const items = useMemo(() => {
    const byId = new Map<number, { item: InventoryItem; owners: string[] }>()
    for (const o of owned ?? []) {
      for (const item of o.list) {
        const entry = byId.get(item.id) ?? { item, owners: [] }
        if (!entry.owners.includes(o.account)) entry.owners.push(o.account)
        byId.set(item.id, entry)
      }
    }
    const read = (owned ?? []).filter((o) => !o.error).length
    const q = query.trim().toLowerCase()
    return [...byId.values()].filter(
      (e) =>
        (mode === 'all' || (mode === 'shared' ? e.owners.length === read && read > 1 : e.owners.length === 1)) &&
        (kind === '*' || e.item.kind === kind) &&
        (!q || e.item.name.toLowerCase().includes(q)),
    )
  }, [owned, mode, kind, query])

  // pictures of the first items shown, asked once each
  useEffect(() => {
    const missing = items.slice(0, 120).map((e) => e.item.id).filter((id) => !(id in pictures))
    if (!missing.length) return
    setPictures((p) => ({ ...p, ...Object.fromEntries(missing.map((id) => [id, ''])) }))
    api.assetThumbnails(missing).then((found) => setPictures((p) => ({ ...p, ...found })), () => {})
  }, [items, pictures])

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-start gap-2">
        <div className="min-w-0 flex-1">
          <AccountPicker value={names} onChange={setNames} />
        </div>
        <Button disabled={!names.length || busy} onClick={load}>
          {busy && <Loader2 className="animate-spin" />} {t('Load inventories')}
        </Button>
      </div>
      {owned && (
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-48 flex-1">
            <Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input className="pl-8" value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t('Search items…')} />
          </div>
          <Select value={mode} onValueChange={setMode}>
            <SelectTrigger className="w-44"><SelectValue /></SelectTrigger>
            <SelectContent>{MODES.map((m) => <SelectItem key={m.value} value={m.value}>{t(m.label)}</SelectItem>)}</SelectContent>
          </Select>
          <Select value={kind} onValueChange={setKind}>
            <SelectTrigger className="w-48"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="*">{t('Every kind')}</SelectItem>
              {KINDS.map(([value, label]) => <SelectItem key={value} value={value}>{t(label)}</SelectItem>)}
            </SelectContent>
          </Select>
          <span className="text-xs text-muted-foreground">{t('{0} items', items.length)}</span>
        </div>
      )}
      <ScrollArea className="min-h-0 flex-1 pr-3">
        <div className="grid grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-2">
          {items.map(({ item, owners }) => (
            <button key={item.id} className="grid cursor-pointer gap-1.5 rounded-lg border p-2 text-left hover:bg-accent" title={item.name} onClick={() => api.open('asset', String(item.id))} data-testid="inventory-item">
              {pictures[item.id] ? <img src={pictures[item.id]} alt="" className="aspect-square w-full rounded-md bg-muted" /> : <span className="aspect-square w-full rounded-md bg-muted" />}
              <span className="truncate text-xs font-medium">{item.name}</span>
              {names.length > 1 && <span className="truncate text-[11px] text-muted-foreground">{owners.join(', ')}</span>}
            </button>
          ))}
        </div>
        {owned === null && !busy && <p className="py-12 text-center text-sm text-muted-foreground">{t('Choose accounts and load their inventories.')}</p>}
        {owned !== null && items.length === 0 && <p className="py-12 text-center text-sm text-muted-foreground">{t('Nothing here')}</p>}
      </ScrollArea>
    </div>
  )
}
