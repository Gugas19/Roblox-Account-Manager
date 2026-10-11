/** App wide state: accounts, selection, settings, theme, open windows, and the events Rust sends */

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import { api, on } from './api'
import { useI18n } from './i18n'
import { applyTheme } from './theme'
import type { Account, Bootstrap, Settings, Theme } from './types'

export type Panel = 'settings' | 'theme' | 'utilities' | 'accountUtils' | 'import' | 'fields' | 'antiafk' | null

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
  const [job, setJob] = useState('')
  const [panel, setPanel] = useState<Panel>(null)
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

  // the selected account's saved place fills the boxes, like the C# version
  useEffect(() => {
    if (selectedAccounts.length !== 1) return
    const a = selectedAccounts[0]
    if (a.savedPlaceId) setPlace(a.savedPlaceId)
    if (a.savedJobId) setJob(a.savedJobId)
  }, [selectedAccounts])

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
    launching,
    download,
    hideUsernames: flag('General', 'HideUsernames'),
    setHideUsernames: (on) => updateSetting('General', 'HideUsernames', on ? 'true' : 'false'),
    devMode: flag('Developer', 'DevMode'),
  }

  return <Context.Provider value={store}>{children}</Context.Provider>
}
