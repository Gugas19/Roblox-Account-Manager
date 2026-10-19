/** A page of the navigation rail: a title, what it's for, and its content filling the window. Esc goes back to the
 *  accounts (when no window is open over it and nothing is being typed). */

import { useEffect, type ReactNode } from 'react'
import { useT } from '@/lib/i18n'
import { useStore, type Page as PageName } from '@/lib/store'

export function Page({ name, title, description, actions, children }: { name: PageName; title: string; description?: string; actions?: ReactNode; children: ReactNode }) {
  const t = useT()
  const { panel, setPage } = useStore()

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
      // a menu, select or dialog handles its own Esc (a tooltip doesn't count)
      const overlay = document.querySelector('[role=dialog], [role=menu], [role=listbox]')
      if (e.key === 'Escape' && !typing && !panel && !overlay) setPage('accounts')
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [panel, setPage])

  return (
    <section data-page={name} aria-label={t(title)} className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card">
      <header className="flex items-start gap-4 border-b px-5 py-4">
        <div className="min-w-0 flex-1">
          <h1 className="text-lg font-semibold">{t(title)}</h1>
          {description && <p className="text-sm text-muted-foreground">{t(description)}</p>}
        </div>
        {actions}
      </header>
      <div className="flex min-h-0 flex-1 flex-col px-5 py-4">{children}</div>
    </section>
  )
}
