/** The window: protection screens first, then the navigation rail and its pages (the account list and the side panel
 *  first), and the windows that open over them */

import { useEffect, useRef, useState } from 'react'
import { Toaster } from '@/components/ui/sonner'
import { TooltipProvider } from '@/components/ui/tooltip'
import { AccountList } from '@/components/accounts/AccountList'
import { AntiAfkPage } from '@/components/antiafk/AntiAfkPage'
import { AccountUtilsDialog } from '@/components/dialogs/AccountUtilsDialog'
import { DialogHost } from '@/components/dialogs/DialogHost'
import { WhatsNewHost } from '@/components/dialogs/WhatsNew'
import { FieldsDialog } from '@/components/dialogs/FieldsDialog'
import { ImportDialog } from '@/components/dialogs/ImportDialog'
import { CommandPalette } from '@/components/layout/CommandPalette'
import { NavRail } from '@/components/layout/NavRail'
import { TopBar } from '@/components/layout/TopBar'
import { ProtectionScreen } from '@/components/protection/ProtectionScreen'
import { SettingsPage } from '@/components/settings/SettingsPage'
import { UpdateChoice } from '@/components/settings/UpdateChoice'
import { Sidebar } from '@/components/sidebar/Sidebar'
import { AvatarPage } from '@/components/utilities/AvatarPage'
import { GamesPage } from '@/components/utilities/GamesPage'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { I18nProvider, useT } from '@/lib/i18n'
import { StoreProvider, useStore } from '@/lib/store'
import type { Bootstrap } from '@/lib/types'

/** Keyboard: Ctrl+K palette, Ctrl+F search, Ctrl+A select all, Enter join, Delete remove, Esc clear selection */
function useShortcuts(searchRef: React.RefObject<HTMLInputElement | null>, openPalette: () => void) {
  const actions = useActions()
  const { panel, page } = useStore()

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
      } else if (!typing && !panel && page === 'accounts') {
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
  }, [actions, panel, page, searchRef, openPalette])
}

/** The right panel's width: dragged with the divider, kept in RAMSettings.ini, double click for the default */
const SIDEBAR_DEFAULT = 360
const SIDEBAR_MIN = 320
const LIST_MIN = 360
/** the navigation rail on the left, and the 16 px gutter on the right */
const RAIL = 56

function clampSidebar(width: number) {
  return Math.round(Math.max(SIDEBAR_MIN, Math.min(width, window.innerWidth - RAIL - 16 - 12 - LIST_MIN)))
}

function useSidebarWidth() {
  const { setting, updateSetting } = useStore()
  const saved = Number(setting('General', 'SidebarWidth')) || SIDEBAR_DEFAULT
  const [width, setWidth] = useState(saved)
  // a smaller window keeps the list usable
  const [, redraw] = useState(0)
  useEffect(() => {
    const onResize = () => redraw((n) => n + 1)
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])
  const save = (w: number) => void updateSetting('General', 'SidebarWidth', String(clampSidebar(w)))
  return { width: clampSidebar(width), setWidth, save }
}

function PanelDivider({ sidebar }: { sidebar: ReturnType<typeof useSidebarWidth> }) {
  const t = useT()
  const start = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault()
    const handle = e.currentTarget
    handle.setPointerCapture(e.pointerId)
    let last = sidebar.width
    const move = (ev: PointerEvent) => {
      // the panel ends 16 px from the right edge, the divider sits in the middle of its 12 px
      last = clampSidebar(window.innerWidth - 16 - ev.clientX - 6)
      sidebar.setWidth(last)
    }
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
      sidebar.save(last)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={t('Drag to resize the panels, double click for the default size')}
      title={t('Drag to resize the panels, double click for the default size')}
      className="group flex w-3 shrink-0 cursor-col-resize touch-none items-center justify-center"
      onPointerDown={start}
      onDoubleClick={() => {
        sidebar.setWidth(SIDEBAR_DEFAULT)
        sidebar.save(SIDEBAR_DEFAULT)
      }}
    >
      <div className="h-10 w-1 rounded-full bg-border opacity-0 transition-opacity group-hover:opacity-100 group-active:opacity-100" />
    </div>
  )
}

function Main() {
  const { setAccounts, page } = useStore()
  const sidebar = useSidebarWidth()
  const sidebarWidth = sidebar.width
  const searchRef = useRef<HTMLInputElement>(null)
  const [palette, setPalette] = useState(false)

  useEffect(() => void api.accountsReady().then(setAccounts), [setAccounts])
  useShortcuts(searchRef, () => setPalette(true))

  return (
    <div className="flex h-screen flex-col bg-background">
      <TopBar onPalette={() => setPalette(true)} />
      <main className="flex min-h-0 flex-1 pr-4 pb-4">
        <NavRail />
        {page === 'accounts' ? (
          <>
            <div className="@container min-h-0 min-w-0 flex-1 overflow-hidden rounded-xl border bg-card">
              <AccountList searchRef={searchRef} />
            </div>
            <PanelDivider sidebar={sidebar} />
            <div className="min-h-0 shrink-0 overflow-hidden rounded-xl border bg-card" style={{ width: sidebarWidth }}>
              <Sidebar />
            </div>
          </>
        ) : page === 'games' ? (
          <GamesPage />
        ) : page === 'avatar' ? (
          <AvatarPage />
        ) : page === 'antiafk' ? (
          <AntiAfkPage />
        ) : (
          <SettingsPage />
        )}
      </main>

      <AccountUtilsDialog />
      <ImportDialog />
      <FieldsDialog />
      <CommandPalette open={palette} onOpenChange={setPalette} />
      <UpdateChoice />
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
            <WhatsNewHost />
            <Toaster position="bottom-right" richColors closeButton />
          </DialogHost>
        </TooltipProvider>
      </StoreProvider>
    </I18nProvider>
  )
}
