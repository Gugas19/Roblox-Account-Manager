/** Several accounts to work on, as chips: click to add or take one away, or all at once */

import { Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function AccountPicker({ value, onChange }: { value: string[]; onChange: (names: string[]) => void }) {
  const t = useT()
  const { accounts } = useStore()
  const all = accounts.length > 0 && accounts.every((a) => value.includes(a.username))
  const toggle = (name: string) => onChange(value.includes(name) ? value.filter((n) => n !== name) : [...value, name])

  return (
    <div className="flex flex-wrap items-center gap-1.5" data-testid="account-picker">
      <Button variant={all ? 'secondary' : 'ghost'} size="xs" onClick={() => onChange(all ? [] : accounts.map((a) => a.username))}>
        {t('All accounts')}
      </Button>
      {accounts.map((a) => {
        const on = value.includes(a.username)
        return (
          <button
            key={a.username}
            aria-pressed={on}
            onClick={() => toggle(a.username)}
            className={`flex cursor-pointer items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs transition-colors ${on ? 'border-primary/50 bg-primary/15 text-foreground' : 'text-muted-foreground hover:bg-accent'}`}
          >
            {on && <Check className="size-3" />}
            {a.username}
          </button>
        )
      })}
    </div>
  )
}
