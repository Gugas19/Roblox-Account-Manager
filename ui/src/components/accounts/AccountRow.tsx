/** One account: status dots, username, alias (double click to edit) and description */

import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useT } from '@/lib/i18n'
import type { Account } from '@/lib/types'

const PRESENCE = [
  null,
  { color: 'bg-sky-500', label: 'Online' },
  { color: 'bg-emerald-500', label: 'In game' },
  { color: 'bg-violet-500', label: 'In Studio' },
]

/** Yellow at 20 days, red 10 days later, like the C# aging alert */
function agingColor(days: number) {
  const k = Math.min(1, Math.max(0, (days - 20) / 10))
  const mix = (a: number, b: number) => Math.round(a + (b - a) * k)
  return `rgb(${mix(255, 250)}, ${mix(204, 26)}, ${mix(77, 13)})`
}

interface Props {
  account: Account
  selected: boolean
  hideUsername: boolean
  showAging: boolean
  showPresence: boolean
  dropTarget: boolean
  onSelect: (e: React.MouseEvent) => void
  onContext: () => void
  onRename: (alias: string) => void
  onDragStart: (e: React.DragEvent) => void
  onDragOver: (e: React.DragEvent) => void
  onDrop: (e: React.DragEvent) => void
  onDragLeave: () => void
  onJoin: () => void
}

export function AccountRow({ account, selected, hideUsername, showAging, showPresence, dropTarget, onSelect, onContext, onRename, onDragStart, onDragOver, onDrop, onDragLeave, onJoin }: Props) {
  const t = useT()
  const [editing, setEditing] = useState(false)
  const [alias, setAlias] = useState(account.alias)
  const aging = showAging && account.daysUnused > 20
  const presence = showPresence ? PRESENCE[account.presence] : null
  // its own color, otherwise its group's
  const bar = account.color ?? account.groupColor

  return (
    <div
      role="row"
      aria-selected={selected}
      draggable={!editing}
      onClick={onSelect}
      onDoubleClick={onJoin}
      onContextMenu={onContext}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onDragLeave={onDragLeave}
      className={`group relative grid h-9 cursor-pointer grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)_minmax(0,1.6fr)] items-center gap-3 rounded-md px-3 text-sm select-none ${
        selected ? 'bg-primary/15 text-foreground ring-1 ring-primary/40' : 'hover:bg-accent'
      } ${dropTarget ? 'shadow-[inset_0_2px_0_0_var(--primary)]' : ''}`}
    >
      {bar && <span aria-hidden className="absolute top-1.5 bottom-1.5 left-0.5 w-1 rounded-full" style={{ background: bar }} />}
      <div className="flex min-w-0 items-center gap-2">
        <span className="flex w-4 shrink-0 gap-0.5">
          {aging && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="size-2 rounded-full" style={{ background: agingColor(account.daysUnused) }} />
              </TooltipTrigger>
              <TooltipContent>{t('Not used for {0} days', Math.floor(account.daysUnused))}</TooltipContent>
            </Tooltip>
          )}
          {presence && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className={`size-2 rounded-full ${presence.color}`} />
              </TooltipTrigger>
              <TooltipContent>{t(presence.label)}</TooltipContent>
            </Tooltip>
          )}
        </span>
        <span className="truncate font-medium" style={account.color ? { color: account.color } : undefined}>{hideUsername ? '••••••••' : account.username}</span>
        {account.fpsLimit != null && <span className="shrink-0 rounded bg-muted px-1 text-[10px] text-muted-foreground tabular-nums" title={t('FPS limit')}>{t('{0} FPS', account.fpsLimit)}</span>}
      </div>

      <div className="min-w-0" onDoubleClick={(e) => { e.stopPropagation(); setAlias(account.alias); setEditing(true) }}>
        {editing ? (
          <Input
            autoFocus
            className="h-7"
            value={alias}
            maxLength={50}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setAlias(e.target.value)}
            onBlur={() => setEditing(false)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                onRename(alias)
                setEditing(false)
              }
              if (e.key === 'Escape') setEditing(false)
              e.stopPropagation()
            }}
          />
        ) : (
          <span className="block truncate text-muted-foreground" title={t('Double click to edit')}>{account.alias || '—'}</span>
        )}
      </div>

      <span className="truncate text-muted-foreground">{account.description.split('\n')[0]}</span>
    </div>
  )
}
