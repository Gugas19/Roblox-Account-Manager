/** Color choices inside a right click submenu: swatches, a custom color, no color */

import { Ban, Pipette } from 'lucide-react'
import { ContextMenuItem, ContextMenuSeparator } from '@/components/ui/context-menu'
import { useT } from '@/lib/i18n'

/** Readable on the dark and the light themes */
export const COLORS = [
  ['Red', '#ef4444'],
  ['Orange', '#f97316'],
  ['Yellow', '#eab308'],
  ['Green', '#22c55e'],
  ['Teal', '#14b8a6'],
  ['Sky', '#0ea5e9'],
  ['Blue', '#3b82f6'],
  ['Purple', '#8b5cf6'],
  ['Pink', '#ec4899'],
  ['Gray', '#94a3b8'],
] as const

interface Props {
  current: string | null
  onPick: (color: string | null) => void
  onCustom: () => void
}

export function ColorItems({ current, onPick, onCustom }: Props) {
  const t = useT()
  return (
    <>
      <div className="grid grid-cols-5 gap-0.5 p-1">
        {COLORS.map(([name, hex]) => (
          <ContextMenuItem key={hex} aria-label={t(name)} title={t(name)} className="justify-center p-1.5" onClick={() => onPick(hex)}>
            <span className={`size-5 rounded-full ${current === hex ? 'ring-2 ring-foreground ring-offset-2 ring-offset-popover' : ''}`} style={{ background: hex }} />
          </ContextMenuItem>
        ))}
      </div>
      <ContextMenuSeparator />
      <ContextMenuItem onClick={onCustom}>
        <Pipette /> {t('Custom color…')}
        {current && !COLORS.some(([, hex]) => hex === current) && <span className="ml-auto size-3 rounded-full" style={{ background: current }} />}
      </ContextMenuItem>
      <ContextMenuItem disabled={!current} onClick={() => onPick(null)}>
        <Ban /> {t('No color')}
      </ContextMenuItem>
    </>
  )
}
