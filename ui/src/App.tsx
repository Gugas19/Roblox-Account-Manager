/** The window: protection screens first, then the account list, the side panel and the windows that open over them */

import { useEffect, useRef, useState } from 'react'
import { Toaster } from '@/components/ui/sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { AccountList } from '@/components/accounts/AccountList'
import { AntiAfkDialog } from '@/components/antiafk/AntiAfkDialog'
import { AccountUtilsDialog } from '@/components/dialogs/AccountUtilsDialog'
import { DialogHost } from '@/components/dialogs/DialogHost'
import { FieldsDialog } from '@/components/dialogs/FieldsDialog'
import { ImportDialog } from '@/components/dialogs/ImportDialog'
import { CommandPalette } from '@/components/layout/CommandPalette'
import { TopBar } from '@/components/layout/TopBar'
import { ProtectionScreen } from '@/components/protection/ProtectionScreen'
import { SettingsDialog } from '@/components/settings/SettingsDialog'
import { ThemeDialog } from '@/components/settings/ThemeDialog'
import { Sidebar } from '@/components/sidebar/Sidebar'
import { UtilitiesDialog } from '@/components/utilities/UtilitiesDialog'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { I18nProvider } from '@/lib/i18n'
import { StoreProvider, useStore } from '@/lib/store'
import type { Bootstrap } from '@/lib/types'

/** Keyboard: Ctrl+K palette, Ctrl+F search, Ctrl+A select all, Enter join, Delete remove, Esc clear selection */
function useShortcuts(searchRef: React.RefObject<HTMLInputElement | null>, openPalette: () => void) {
  const actions = useActions()
  const { panel } = useStore()

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || (e.target as HTMLElement)?.isContentEditable
      if (e.ctrlKey && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        openPalette()
      } else if (e.ctrlKey && e.key.toLowerCase() === 'f') {
        e.preventDefault()
        searchRef.current?.focus()
        searchRef.current?.select()
      } else if (!typing && !panel) {
        if (e.ctrlKey && e.key.toLowerCase() === 'a') {
          e.preventDefault()
          actions.selectAll()
        } else if (e.key === 'Delete') actions.remove()
        else if (e.key === 'Enter') actions.join()
        else if (e.key === 'Escape') actions.clearSelection()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [actions, panel, searchRef, openPalette])
}

function Main() {
  const { setAccounts } = useStore()
  const searchRef = useRef<HTMLInputElement>(null)
  const [palette, setPalette] = useState(false)

  useEffect(() => void api.accountsReady().then(setAccounts), [setAccounts])
  useShortcuts(searchRef, () => setPalette(true))

  return (
    <div className="flex h-screen flex-col bg-background">
      <TopBar onPalette={() => setPalette(true)} />
      <main className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-3 px-4 pb-4">
        <div className="min-h-0 overflow-hidden rounded-xl border bg-card">
          <AccountList searchRef={searchRef} />
        </div>
        <div className="min-h-0 overflow-hidden rounded-xl border bg-card">
          <Sidebar />
        </div>
      </main>

      <SettingsDialog />
      <ThemeDialog />
      <UtilitiesDialog />
      <AccountUtilsDialog />
      <ImportDialog />
      <FieldsDialog />
      <AntiAfkDialog />
      <CommandPalette open={palette} onOpenChange={setPalette} />
    </div>
  )
}

function Gate({ boot }: { boot: Bootstrap }) {
  const [ready, setReady] = useState(boot.loadState.state === 'ready')
  return ready ? <Main /> : <ProtectionScreen state={boot.loadState} onReady={() => setReady(true)} />
}

export default function App({ boot }: { boot: Bootstrap }) {
  return (
    <I18nProvider language={boot.language} table={boot.table}>
      <StoreProvider boot={boot}>
        <TooltipProvider delayDuration={400}>
          <DialogHost>
            <Gate boot={boot} />
            <Toaster position="bottom-right" richColors closeButton />
          </DialogHost>
        </TooltipProvider>
      </StoreProvider>
    </I18nProvider>
  )
}
