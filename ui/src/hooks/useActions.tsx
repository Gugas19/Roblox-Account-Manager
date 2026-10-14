/**
 * Every action of the app in one place. The toolbar, the account menu, the command palette and the keyboard shortcuts
 * all call these, so each action behaves the same wherever it's started from.
 */

import { useCallback } from 'react'
import { toast } from 'sonner'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { JoinResult } from '@/lib/types'

export type CopyKind = 'username' | 'password' | 'combo' | 'profile' | 'userId' | 'token' | 'group' | 'ticket' | 'playerLink' | 'appLink'

export function useActions() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const store = useStore()
  const { selected, selectedAccounts, place, job } = store

  const needSelection = useCallback(() => {
    if (selected.length === 0) {
      notify.info('No Account Selected!')
      return false
    }
    return true
  }, [selected.length, notify])

  /** A join that may need the user's ok to open the account's private server privacy */
  const handleJoin = useCallback(async (first: JoinResult, retry: () => Promise<JoinResult>, account: string) => {
    let result = first
    while (result.result === 'needsPrivacy') {
      const ok = await dialogs.confirm({
        title: 'Account Manager has detected your account\'s privacy settings do not allow you to join private servers.',
        description: 'Would you like to change this setting to Everyone now?',
      })
      if (!ok) return
      try {
        await api.allowPrivateServers(account)
      } catch (e) {
        notify.error(e)
      }
      result = await retry()
    }
    if (result.result === 'error') notify.error(result.message)
  }, [dialogs, notify])

  return {
    // ---------- adding ----------
    addAccount: async () => {
      await api.addAccountBrowser()
      notify.info('Log in to Roblox in the browser window, the account is added as soon as you do')
    },
    importCookies: () => store.setPanel('import'),
    addUserPass: async () => {
      const text = await dialogs.prompt({ title: 'Import by User:Pass', label: 'Separate the accounts with new lines\nMust be in user:pass form', multiline: true, placeholder: 'username:password' })
      if (text?.trim()) await api.addUserPass(text)
    },
    customBrowser: async () => {
      const url = await dialogs.prompt({ title: 'Launch Browser', label: 'URL', defaultValue: 'https://roblox.com/' })
      if (!url) return
      const script = await dialogs.prompt({ title: 'Launch Browser', label: 'Javascript', multiline: true })
      if (script === null) return
      await api.customBrowsers(url, script, 1)
    },

    // ---------- the selection ----------
    remove: async () => {
      if (!needSelection()) return
      const names = [...selected]
      await api.removeAccounts(names)
      toast(names.length === 1 ? t('Removed {0}', names[0]) : t('Removed {0} accounts', names.length), {
        action: { label: t('Undo'), onClick: () => void api.undoRemove() },
        duration: 8000,
      })
    },
    selectAll: () => store.setSelected(store.accounts.map((a) => a.username)),
    clearSelection: () => store.setSelected([]),

    setAlias: async (alias: string, names = selected) => {
      try {
        await api.setAlias(names, alias)
        notify.success('Saved')
      } catch (e) {
        notify.error(e)
      }
    },
    setDescription: async (description: string) => {
      if (!needSelection()) return
      try {
        await api.setDescription(selected, description)
        notify.success('Saved')
      } catch (e) {
        notify.error(e)
      }
    },
    moveToGroup: async () => {
      if (!needSelection()) return
      const group = await dialogs.prompt({ title: 'Move Account to Group', label: 'Group Name', defaultValue: selectedAccounts[0]?.group ?? '' })
      if (group !== null) await api.moveToGroup(selected, group)
    },
    sortAlphabetically: async () => {
      if (await dialogs.confirm({ title: 'Are you sure you want to sort every account alphabetically?' })) await api.sortAlphabetically()
    },
    copy: async (kind: CopyKind) => {
      if (!needSelection()) return
      try {
        await navigator.clipboard.writeText(await api.copyText(selected, kind, place, job))
        notify.success('Copied')
      } catch (e) {
        notify.error(e)
      }
    },
    dumpDetails: () => needSelection() && api.dumpDetails(selected),
    viewFields: () => needSelection() && store.setPanel('fields'),
    quickLogIn: async () => {
      const name = selected[0]
      if (!needSelection()) return
      const understood = await dialogs.confirm({ title: 'Only enter codes that you requested\nNever enter another user\'s code', description: 'Do you understand?' })
      if (!understood) return
      const code = await dialogs.prompt({ title: 'Quick Log In', label: 'Code', placeholder: 'ABC123' })
      if (!code) return
      try {
        const info = await api.quickLoginInfo(name, code)
        const ok = await dialogs.confirm({ title: 'Please confirm you are logging in with this device', description: `${t('Device: {0}', info.device)}\n${t('Location: {0}', info.location)}` })
        if (ok && (await api.quickLoginConfirm(name, code))) notify.success('Logged in')
      } catch (e) {
        notify.error(e)
      }
    },

    // ---------- browser ----------
    openBrowser: async (mode: 'home' | 'url' | 'script' | 'group' = 'home') => {
      if (!needSelection()) return
      try {
        if (mode === 'home') return await api.openBrowser(selected)
        if (mode === 'script') {
          const ok = await dialogs.confirm({ title: 'Your accounts may be at risk using this feature', description: 'Do not paste in javascript unless you know what it does, your account\'s cookies can easily be logged through javascript.\n\nPress Yes to continue', destructive: true })
          if (!ok) return
        }
        const url = await dialogs.prompt({ title: 'Open Browser', label: mode === 'group' ? 'Group Link' : 'URL', placeholder: 'https://www.roblox.com/…' })
        if (!url) return
        const script = mode === 'script' ? await dialogs.prompt({ title: 'Open Browser', label: 'Javascript', multiline: true }) : null
        await api.openBrowser(selected, url, script, mode === 'group')
      } catch (e) {
        notify.error(e)
      }
    },

    // ---------- games ----------
    join: async () => {
      if (!needSelection()) return
      const names = [...selected]
      const run = () => api.join(names, place, job)
      try {
        const result = await run()
        await handleJoin(result, () => api.join(names, place, job, true), names[0])
      } catch (e) {
        notify.error(e)
      }
    },
    follow: async (username: string) => {
      if (!needSelection() || !username.trim()) return
      try {
        const { userId, inGame } = await api.followCheck(username)
        if (!inGame) {
          const ok = await dialogs.confirm({ title: 'The user you are trying to follow is not in game or has their joins off', description: 'Do you want to attempt to join anyways?' })
          if (!ok) return
        }
        const result = await api.follow(selected, userId)
        if (result.result === 'error') notify.error(result.message)
      } catch (e) {
        notify.error(e)
      }
    },
    clearSavedPlaces: async () => {
      if (await dialogs.confirm({ title: 'Remove every saved Place Id?', destructive: true })) notify.success(t('Removed {0} saved Place Ids', await api.clearSavedPlaces()))
    },

    // ---------- windows ----------
    open: store.setPanel,
  }
}

export type Actions = ReturnType<typeof useActions>
