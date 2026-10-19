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
  /** the game's name when in a game or Studio, when Roblox says */
  location?: string | null
  savedPlaceId: string
  savedJobId: string
  hasPassword: boolean
  /** the account's own color, "#rrggbb" */
  color: string | null
  /** its group's color */
  groupColor: string | null
  /** its own frame rate limit, applied when it launches */
  /** CPU limit of its Roblox window, percent of the PC */
  cpuLimit: number | null
  /** its tags, several besides its group */
  tags: string[]
  /** pinned to the top of its group */
  pinned: boolean
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
  /** the server box as it was left */
  jobId: string
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
  /** the server it's in, from the Roblox log */
  jobId: string | null
  /** a private server (its access code stays in Rust) */
  private: boolean
  secondsRunning: number
  state: 'playing' | 'disconnected' | 'loading'
  included: boolean
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
/** Something an account owns (src/roblox/inventory.rs) */
export interface InventoryItem {
  id: number
  name: string
  kind: string
}

/** A group an account is in, with its role there */
export interface GroupMembership {
  id: number
  name: string
  members: number
  role: string
  rank: number
}

/** One account's list (items, groups), or why it couldn't be read */
export interface Owned<T> {
  account: string
  list: T[]
  error?: string | null
}

/** A friend (or a friend request) of an account: 0 offline, 1 online, 2 in a game, 3 in Studio */
export interface Friend {
  id: number
  name: string
  displayName: string
  kind: number
  placeId?: number | null
  jobId?: string | null
  location?: string | null
  avatar?: string | null
}

/** Who can join an account / see it online: now, and the values Roblox allows it */
export interface Visibility {
  field: string
  current: string
  options: string[]
}

/** One event of the session history (src/services/history.rs) */
export interface HistoryEvent {
  /** seconds since 1970 */
  at: number
  account: string
  /** joined, server, disconnected, rejoined, closed (this PC); online, offline, playing, studio (presence) */
  kind: string
  placeId?: number | null
  place?: string | null
  jobId?: string | null
}

/** Anti-AFK statistics put aside by a reset (seconds since 1970 for the dates) */
export interface PastStats {
  id: number
  since: number | null
  until: number
  seconds: number
  actions: number
  rejoins: number
  sessions: number
  longest: number
}

/** What changed in one version (its release notes, without the download and verify parts) */
export interface ReleaseNotes {
  version: string
  date: string
  text: string
}

export type UpdateStatus =
  | { state: 'idle' | 'checking' | 'upToDate' | 'disabled' }
  | { state: 'available' | 'ready'; version: string }
  | { state: 'downloading'; version: string; progress: number }
  | { state: 'error'; message: string }
