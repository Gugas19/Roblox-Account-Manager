/** Shapes the Rust side sends (see src/app.rs and src/commands/) */

export interface Account {
  username: string
  userId: number
  alias: string
  description: string
  group: string
  groupTitle: string
  daysUnused: number
  /** 0 offline, 1 online, 2 in game, 3 in studio */
  presence: number
  savedPlaceId: string
  savedJobId: string
  hasPassword: boolean
  /** the account's own color, "#rrggbb" */
  color: string | null
  /** its group's color */
  groupColor: string | null
}

export type LoadState =
  | { state: 'ready' }
  | { state: 'chooseProtection' }
  | { state: 'needsPassword' }
  | { state: 'offerBackup'; locked: boolean }
  | { state: 'failed'; message: string }

export interface Language {
  code: string
  name: string
  flag: string
}

export type Settings = Record<string, Record<string, string>>

export type Theme = Record<string, string>

export interface Bootstrap {
  language: string
  table: string
  languages: Language[]
  theme: Theme
  settings: Settings
  loadState: LoadState
  multiRobloxOk: boolean
  browserInstalled: boolean
  placeId: string
  followUser: string
  version: string
  startup: boolean
  webServer: string | null
}

export type JoinResult = { result: 'success' } | { result: 'needsPrivacy' } | { result: 'error'; message: string }

export interface Game {
  placeId: number
  name: string
  icon?: string | null
  players?: number
  likes?: number
}

export interface Favorite extends Game {
  index: number
  privateServer: string
}

export interface Server {
  id: string
  accessCode?: string | null
  playing: number
  maxPlayers: number
  ping?: number | null
  fps?: number | null
  type: 'Public' | 'VIP'
  region: string
  playerTokens?: string[]
}

export interface Outfit {
  id: number
  name: string
  icon?: string | null
}

export interface MissingAsset {
  id: number
  name: string
  price: number | null
  productId: number | null
  sellerId: number | null
  forSale: boolean
  icon?: string | null
}

export interface AntiAfkWindow {
  pid: number
  account: string | null
  title: string
  placeId: number | null
  state: 'playing' | 'disconnected' | 'loading'
  included: boolean
  /** keys go to it in the background (experimental mode) */
  background: boolean
}

export interface AntiAfkStatus {
  running: boolean
  secondsToNext: number | null
  idleSeconds: number
  windows: AntiAfkWindow[]
  stats: { seconds: number; actions: number; rejoins: number; sessions: number; longest: number }
  hidden: boolean
  /** waiting for the user to stop typing and moving the mouse, or pressing keys right now */
  phase: 'waiting' | 'acting' | null
}

/** The auto updater (src/services/updater.rs) */
export type UpdateStatus =
  | { state: 'idle' | 'checking' | 'upToDate' | 'disabled' }
  | { state: 'available' | 'ready'; version: string }
  | { state: 'downloading'; version: string; progress: number }
  | { state: 'error'; message: string }
