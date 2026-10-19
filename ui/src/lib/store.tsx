/** App wide state: accounts, selection, settings, theme, the page shown, open windows, and the events Rust sends */

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import { api, on } from './api'
import { useI18n } from './i18n'
import { applyTheme } from './theme'
import type { Account, Bootstrap, Settings, Theme } from './types'

/** The pages of the navigation rail; Accounts is home */
export type Page = 'accounts' | 'instances' | 'friends' | 'groups' | 'games' | 'avatar' | 'antiafk' | 'history' | 'settings'

/** Windows over the page. 'settings', 'theme', 'utilities' and 'antiafk' were windows before the pages, they open
 *  their page (and tab) now, so every way of opening them still works */
export type Panel = 'settings' | 'theme' | 'utilities' | 'accountUtils' | 'import' | 'fields' | 'antiafk' | null

/** Old window → its page and tab */
const PANEL_PAGES: Partial<Record<NonNullable<Panel>, [Page, string?]>> = {
  settings: ['settings'],
  theme: ['settings', 'appearance'],
  utilities: ['games'],
  antiafk: ['antiafk'],
}

interface Store {
  boot: Bootstrap
  accounts: Account[]
  setAccounts: (accounts: Account[]) => void
  selected: string[]
  selectedAccounts: Account[]
  select: (username: string, mode?: 'only' | 'toggle' | 'range') => void
  setSelected: (names: string[]) => void
  settings: Settings
  setting: (section: string, key: string) => string
  flag: (section: string, key: string) => boolean
  updateSetting: (section: string, key: string, value: string) => Promise<void>
  theme: Theme
  updateTheme: (key: string, value: string) => Promise<void>
  resetTheme: () => Promise<void>
  place: string
  setPlace: (place: string) => void
  job: string
  setJob: (job: string) => void
  panel: Panel
  setPanel: (panel: Panel) => void
  page: Page
  /** the tab a page opens on, when something asked for one (e.g. the theme opens Settings > Appearance) */
  pageTab: string | undefined
  setPage: (page: Page, tab?: string) => void
  launching: { total: number; done: number }
  download: number | null
  hideUsernames: boolean
  setHideUsernames: (on: boolean) => void
  devMode: boolean
}

const Context = createContext<Store>(null as unknown as Store)
export const useStore = () => useContext(Context)

export function StoreProvider({ boot, children }: { boot: Bootstrap; children: ReactNode }) {
  const { t } = useI18n()
  const [accounts, setAccountsState] = useState<Account[]>([])
  const [selected, setSelected] = useState<string[]>([])
  const [settings, setSettings] = useState<Settings>(boot.settings)
  const [theme, setTheme] = useState<Theme>(boot.theme)
  const [place, setPlace] = useState(boot.placeId)
  const [job, setJob] = useState(boot.jobId ?? '')
  const [panel, setPanelState] = useState<Panel>(null)
  const [pageState, setPageState] = useState<{ page: Page; tab?: string }>({ page: 'accounts' })
  const setPage = useCallback((page: Page, tab?: string) => {
    setPanelState(null)
    setPageState({ page, tab })
  }, [])
  const setPanel = useCallback(
    (next: Panel) => {
      const target = next ? PANEL_PAGES[next] : undefined
      if (target) setPage(target[0], target[1])
      else setPanelState(next)
    },
    [setPage],
  )
  const [launching, setLaunching] = useState({ total: 0, done: 0 })
  const [download, setDownload] = useState<number | null>(null)
  const anchor = useRef<string | null>(null)

  const setting = useCallback((section: string, key: string) => settings[section]?.[key] ?? '', [settings])
  const flag = useCallback((section: string, key: string) => (settings[section]?.[key] ?? '').toLowerCase() === 'true', [settings])

  const setAccounts = useCallback((list: Account[]) => {
    setAccountsState(list)
    setSelected((current) => current.filter((name) => list.some((a) => a.username === name)))
  }, [])

  useEffect(() => applyTheme(theme), [theme])

  useEffect(() => {
    const stops = [
      on<Account[]>('accounts', setAccounts),
      on<{ kind: string; message: string }>('toast', ({ kind, message }) => (kind === 'error' ? toast.error(t(message), { duration: 8000 }) : toast.success(t(message)))),
      on<{ total: number; done: number }>('launching', setLaunching),
      on<number>('browser-download', (p) => setDownload(p >= 1 || p < 0 ? null : p)),
    ]
    return () => stops.forEach((s) => s.then((stop) => stop()))
  }, [setAccounts, t])

  // the order the list shows them (groups sorted), for shift+click ranges
  const ordered = useMemo(() => [...accounts].sort((a, b) => a.group.localeCompare(b.group, undefined, { numeric: true })), [accounts])

  const select = useCallback((username: string, mode: 'only' | 'toggle' | 'range' = 'only') => {
    if (mode === 'toggle') {
      setSelected((s) => (s.includes(username) ? s.filter((n) => n !== username) : [...s, username]))
      anchor.current = username
    } else if (mode === 'range' && anchor.current) {
      const names = ordered.map((a) => a.username)
      const [from, to] = [names.indexOf(anchor.current), names.indexOf(username)].sort((a, b) => a - b)
      if (from >= 0) setSelected(names.slice(from, to + 1))
    } else {
      setSelected([username])
      anchor.current = username
    }
  }, [ordered])

  const selectedAccounts = useMemo(() => accounts.filter((a) => selected.includes(a.username)), [accounts, selected])

  // Choosing an account fills the boxes with its own game, or with the last one used when it has none, unless the
  // same game is set for every account. Only when the selection changes: the boxes save into the accounts as they're
  // typed in, and that save mustn't fill them again with an older value
  const latest = useRef({ accounts, settings })
  latest.current = { accounts, settings }
  const selectionKey = selected.join('\n')
  useEffect(() => {
    const { accounts, settings } = latest.current
    if (selected.length !== 1 || settings.General?.SameGameForAll === 'true') return
    const a = accounts.find((x) => x.username === selected[0])
    if (!a) return
    if (a.savedPlaceId || a.savedJobId) {
      setPlace(a.savedPlaceId ?? '')
      setJob(a.savedJobId ?? '')
    } else {
      setPlace(settings.General?.SavedPlaceId ?? '')
      setJob(settings.General?.SavedJobId ?? '')
    }
  }, [selectionKey]) // eslint-disable-line react-hooks/exhaustive-deps

  const updateSetting = useCallback(async (section: string, key: string, value: string) => {
    setSettings((s) => ({ ...s, [section]: { ...s[section], [key]: value } }))
    try {
      const message = await api.setSetting(section, key, value)
      if (message) toast(t(message))
    } catch (error) {
      toast.error(t(error instanceof Error ? error.message : String(error)), { duration: 8000 })
    }
  }, [t])

  const updateTheme = useCallback(async (key: string, value: string) => setTheme(await api.setTheme(key, value)), [])
  const resetTheme = useCallback(async () => setTheme(await api.resetTheme()), [])

  const store: Store = {
    boot,
    accounts,
    setAccounts,
    selected,
    selectedAccounts,
    select,
    setSelected,
    settings,
    setting,
    flag,
    updateSetting,
    theme,
    updateTheme,
    resetTheme,
    place,
    setPlace,
    job,
    setJob,
    panel,
    setPanel,
    page: pageState.page,
    pageTab: pageState.tab,
    setPage,
    launching,
    download,
    hideUsernames: flag('General', 'HideUsernames'),
    setHideUsernames: (on) => updateSetting('General', 'HideUsernames', on ? 'true' : 'false'),
    devMode: flag('Developer', 'DevMode'),
  }

  return <Context.Provider value={store}>{children}</Context.Provider>
}
