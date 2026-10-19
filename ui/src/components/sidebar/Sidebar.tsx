/** Right side: the game (following a player is in the Join menu), then the selected account: its alias,
 *  description and utilities */

import { Check, UserCog } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { Textarea } from '@/components/ui/textarea'
import { GamePanel } from '@/components/sidebar/GamePanel'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

function Heading({ children }: { children: React.ReactNode }) {
  return <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{children}</div>
}

type Field = 'alias' | 'description'

export function Sidebar() {
  const t = useT()
  const notify = notifier(t)
  const { selected, selectedAccounts, setPanel } = useStore()
  const [alias, setAlias] = useState('')
  const [description, setDescription] = useState('')
  const [saved, setSaved] = useState(false)
  const one = selectedAccounts.length === 1 ? selectedAccounts[0] : null
  const none = selectedAccounts.length === 0

  // Alias and description save by themselves a moment after typing stops, into the accounts selected while typing.
  // Choosing other accounts first saves what's still waiting.
  const pending = useRef<{ field: Field; value: string; names: string[]; timer: ReturnType<typeof setTimeout> }[]>([])
  const save = async (field: Field, value: string, names: string[]) => {
    try {
      await (field === 'alias' ? api.setAlias(names, value) : api.setDescription(names, value))
      setSaved(true)
    } catch (e) {
      notify.error(e)
    }
  }
  const flush = () => {
    for (const p of pending.current.splice(0)) {
      clearTimeout(p.timer)
      void save(p.field, p.value, p.names)
    }
  }
  const change = (field: Field, value: string) => {
    ;(field === 'alias' ? setAlias : setDescription)(value)
    setSaved(false)
    const names = [...selected]
    const waiting = pending.current.find((p) => p.field === field)
    if (waiting) clearTimeout(waiting.timer)
    const entry = { field, value, names, timer: setTimeout(() => {
      pending.current = pending.current.filter((p) => p !== entry)
      void save(field, value, names)
    }, 600) }
    pending.current = [...pending.current.filter((p) => p.field !== field), entry]
  }

  const selection = selected.join('\n')
  useEffect(() => {
    flush()
    setSaved(false)
    return flush
  }, [selection]) // eslint-disable-line react-hooks/exhaustive-deps

  // what the accounts have (changed elsewhere too, like renaming in the list), unless it's being typed
  useEffect(() => {
    if (!pending.current.some((p) => p.field === 'alias')) setAlias(one?.alias ?? '')
  }, [one?.username, one?.alias])
  useEffect(() => {
    if (!pending.current.some((p) => p.field === 'description')) setDescription(one?.description ?? '')
  }, [one?.username, one?.description])

  return (
    <aside className="flex h-full min-h-0 flex-col gap-4 overflow-y-auto p-4">
      <GamePanel />

      <section className="mt-2 grid flex-1 grid-rows-[auto_auto_minmax(5rem,1fr)] gap-2 border-t pt-4">
        <div className="flex items-center justify-between gap-2">
          <Heading>{none ? t('SELECTED ACCOUNT') : one ? one.username : t('{0} selected', selectedAccounts.length)}</Heading>
          <div className="flex items-center gap-1">
            {saved && (
              <span className="flex items-center gap-1 text-xs text-muted-foreground" data-testid="saved">
                <Check className="size-3.5 text-emerald-500" /> {t('Saved')}
              </span>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon-sm" aria-label={t('Account Utilities')} disabled={selectedAccounts.length !== 1} onClick={() => setPanel('accountUtils')}>
                  <UserCog />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('Account Utilities')}</TooltipContent>
            </Tooltip>
          </div>
        </div>
        <Input value={alias} maxLength={50} disabled={none} onChange={(e) => change('alias', e.target.value)} placeholder={t('Alias')} />
        <Textarea className="min-h-20 resize-none" value={description} maxLength={5000} disabled={none} onChange={(e) => change('description', e.target.value)} placeholder={t('Description')} />
      </section>

    </aside>
  )
}
