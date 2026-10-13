/** Typed calls to the Rust commands (src/commands/*.rs), one function per command */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Account, AntiAfkStatus, Bootstrap, Favorite, Game, JoinResult, LoadState, MissingAsset, Outfit, Settings, Theme, UpdateStatus } from './types'

/** Errors from Rust arrive as plain strings */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    throw new Error(typeof error === 'string' ? error : String(error))
  }
}

export const api = {
  // start up & protection
  bootstrap: () => call<Bootstrap>('bootstrap'),
  accountsReady: () => call<Account[]>('accounts_ready'),
  chooseProtection: (password: string | null) => call<void>('choose_protection', { password }),
  unlock: (password: string) => call<void>('unlock', { password }),
  checkPassword: (password: string) => call<void>('check_password', { password }),
  protection: () => call<{ hasPassword: boolean; protection: 'Default' | 'Password' | 'None' }>('protection'),
  loadBackup: () => call<LoadState>('load_backup'),

  // accounts
  listAccounts: () => call<Account[]>('list_accounts'),
  removeAccounts: (names: string[]) => call<void>('remove_accounts', { names }),
  undoRemove: () => call<number>('undo_remove'),
  setAlias: (names: string[], alias: string) => call<void>('set_alias', { names, alias }),
  setDescription: (names: string[], description: string) => call<void>('set_description', { names, description }),
  setAccountColor: (names: string[], color: string | null) => call<void>('set_account_color', { names, color }),
  setFpsLimit: (names: string[], limit: number | null) => call<void>('set_fps_limit', { names, limit }),
  renameGroup: (from: string, to: string) => call<number>('rename_group', { from, to }),
  exportAccounts: (password: string) => call<string | null>('export_accounts', { password }),
  importBackup: (path: string, password: string) => call<{ added: number; existing: number }>('import_backup', { path, password }),
  setGroupColor: (group: string, color: string | null) => call<void>('set_group_color', { group, color }),
  moveToGroup: (names: string[], group: string) => call<void>('move_to_group', { names, group }),
  reorder: (names: string[], before: string | null, group: string) => call<void>('reorder', { names, before, group }),
  sortAlphabetically: () => call<void>('sort_alphabetically'),
  savePlace: (names: string[], placeId: string, jobId: string) => call<void>('save_place', { names, placeId, jobId }),
  clearSavedPlaces: () => call<number>('clear_saved_places'),
  importCookies: (text: string) => call<{ text: string; ok: boolean }[]>('import_cookies', { text }),
  accountFields: (name: string) => call<Record<string, string>>('account_fields', { name }),
  setField: (name: string, field: string, value: string) => call<void>('set_field', { name, field, value }),
  copyText: (names: string[], kind: string, placeId?: string, jobId?: string) => call<string>('copy_text', { names, kind, placeId, jobId }),
  dumpDetails: (names: string[]) => call<void>('dump_details', { names }),
  quickLoginInfo: (name: string, code: string) => call<{ device: string; location: string }>('quick_login_info', { name, code }),
  quickLoginConfirm: (name: string, code: string) => call<boolean>('quick_login_confirm', { name, code }),

  // joining
  join: (names: string[], place: string, job: string, privacyOk = false) => call<JoinResult>('join', { names, place, job, privacyOk }),
  cancelLaunch: () => call<void>('cancel_launch'),
  followCheck: (username: string) => call<{ userId: number; inGame: boolean }>('follow_check', { username }),
  follow: (names: string[], userId: number) => call<JoinResult>('follow', { names, userId }),
  allowPrivateServers: (name: string) => call<boolean>('allow_private_servers', { name }),
  placeInfo: (place: string) => call<{ name: string; icon?: string } | null>('place_info', { place }),
  setShuffle: (on: boolean) => call<void>('set_shuffle', { on }),

  // games
  recentGames: () => call<Game[]>('recent_games'),
  searchGames: (term: string, page: number) => call<Game[]>('search_games', { term, page }),
  favorites: () => call<Favorite[]>('favorites'),
  addFavorite: (place: string, job: string, name?: string) => call<void>('add_favorite', { place, job, name }),
  renameFavorite: (index: number, name: string) => call<void>('rename_favorite', { index, name }),
  removeFavorite: (index: number) => call<void>('remove_favorite', { index }),
  universeId: (place: string) => call<number>('universe_id', { place }),
  universePlaces: (universe: number) => call<Game[]>('universe_places', { universe }),

  // servers
  listServers: (place: string, otherPlace: string, account: string | null) => call<number>('list_servers', { place, otherPlace, account }),
  stopServers: () => call<void>('stop_servers'),
  joinJob: (name: string, place: string, job: string, otherPlace: string) => call<JoinResult>('join_job', { name, place, job, otherPlace }),
  loadRegions: (name: string | null, place: string, ids: string[]) => call<void>('load_regions', { name, place, ids }),
  findPlayer: (username: string, place: string, otherPlace: string) => call<number>('find_player', { username, place, otherPlace }),

  // avatar
  outfits: (username: string) => call<Outfit[]>('outfits', { username }),
  outfitJson: (outfitId: number) => call<string>('outfit_json', { outfitId }),
  wear: (name: string, avatar: string) => call<MissingAsset[]>('wear', { name, avatar }),
  purchase: (name: string, productId: number, price: number, sellerId: number) => call<Record<string, unknown>>('purchase', { name, productId, price, sellerId }),

  // account utilities
  changePassword: (name: string, current: string, newPassword: string) => call<string>('change_password', { name, current, new: newPassword }),
  changeEmail: (name: string, password: string, email: string) => call<void>('change_email', { name, password, email }),
  setDisplayName: (name: string, displayName: string) => call<void>('set_display_name', { name, displayName }),
  addFriend: (name: string, username: string) => call<void>('add_friend', { name, username }),
  toggleBlock: (name: string, username: string) => call<boolean>('toggle_block', { name, username }),
  unlockPin: (name: string, pin: string) => call<void>('unlock_pin', { name, pin }),
  setFollowPrivacy: (name: string, privacy: number) => call<void>('set_follow_privacy', { name, privacy }),
  signOutOthers: (name: string) => call<string>('sign_out_others', { name }),

  // browser
  browserStatus: () => call<{ installed: boolean; downloading: boolean; progress: number; folder: string }>('browser_status'),
  addAccountBrowser: () => call<void>('add_account_browser'),
  addUserPass: (text: string) => call<void>('add_user_pass', { text }),
  openBrowser: (names: string[], url: string | null = null, script: string | null = null, joinGroup = false) => call<void>('open_browser', { names, url, script, joinGroup }),
  customBrowsers: (url: string, script: string, count: number) => call<void>('custom_browsers', { url, script, count }),

  // anti-afk
  antiAfkStatus: () => call<AntiAfkStatus>('antiafk_status'),
  antiAfkStart: () => call<void>('antiafk_start'),
  antiAfkStop: () => call<void>('antiafk_stop'),
  antiAfkNow: () => call<void>('antiafk_now'),
  antiAfkTool: (kind: 'grid' | 'hide' | 'show' | 'seeThrough' | 'solid' | 'mute' | 'unmute' | 'closeAll') => call<string>('antiafk_tool', { kind }),
  antiAfkTestWebhook: () => call<void>('antiafk_test_webhook'),
  antiAfkResetStats: () => call<void>('antiafk_reset_stats'),
  updateStatus: () => call<{ status: UpdateStatus; version: string }>('update_status'),
  updateCheck: (download: boolean) => call<UpdateStatus>('update_check', { download }),
  updateRestart: () => call<void>('update_restart'),

  // settings
  getSettings: () => call<Settings>('get_settings'),
  setSetting: (section: string, key: string, value: string) => call<string | null>('set_setting', { section, key, value }),
  setStartup: (enabled: boolean) => call<void>('set_startup', { enabled }),
  setLanguage: (code: string) => call<{ language: string; table: string }>('set_language', { code }),
  setTheme: (key: string, value: string) => call<Theme>('set_theme', { key, value }),
  resetTheme: () => call<Theme>('reset_theme'),
  pickFile: (title: string) => call<string | null>('pick_file', { title }),
  open: (kind: 'discord' | 'github' | 'logs' | 'browser' | 'data' | 'profile' | 'game', value?: string) => call<void>('open_target', { kind, value }),
  watcherEnable: (enabled: boolean) => call<void>('watcher_enable', { enabled }),
}

/** Listens to an event from Rust, returns the function that stops listening */
export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload))
}
