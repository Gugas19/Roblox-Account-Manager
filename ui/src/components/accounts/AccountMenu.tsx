/** Right click menu of the account list (same items as the C# version, developer ones only in developer mode) */

import { ArrowDownAZ, Check, ClipboardCopy, Code, FileText, FolderInput, Gauge, House, Info, KeyRound, LogIn, Palette, PenLine, Pin, PinOff, Tag, Trash2, UserPlus } from 'lucide-react'
import { ContextMenuContent, ContextMenuItem, ContextMenuLabel, ContextMenuSeparator, ContextMenuSub, ContextMenuSubContent, ContextMenuSubTrigger } from '@/components/ui/context-menu'
import { ColorItems } from '@/components/accounts/ColorItems'
import { tagColor } from '@/components/accounts/Tags'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

/** What a custom color is for, picked in the system color picker after the menu closes */
export type ColorTarget = { kind: 'accounts'; names: string[]; current: string | null } | { kind: 'group'; key: string; current: string | null }

export interface MenuGroup {
  key: string
  title: string
  color: string | null
}

interface Props {
  showGroups: boolean
  toggleGroups: () => void
  /** right click on a group's header: the group's menu instead of the accounts' one */
  group: MenuGroup | null
  onCustomColor: (target: ColorTarget) => void
}

/** Frame rate limits offered in the menu (a custom one is possible too) */
/** Percent of the whole PC's CPU, as Task Manager counts it */
const CPU_CHOICES = [3, 5, 10, 15, 25, 50]

export function AccountMenu({ showGroups, toggleGroups, group, onCustomColor }: Props) {
  const t = useT()
  const notify = notifier(t)
  const actions = useActions()
  const dialogs = useDialogs()
  const { devMode, selected, selectedAccounts, accounts } = useStore()
  // every tag in use, for the Tags menu
  const allTags = [...new Set(accounts.flatMap((a) => a.tags))].sort((a, b) => a.localeCompare(b))
  const allPinned = selectedAccounts.length > 0 && selectedAccounts.every((a) => a.pinned)
  const newTag = async () => {
    const tag = await dialogs.prompt({ title: 'New tag', label: 'A word to find and tell the accounts apart, like Farm or Trading (up to 24 characters).' })
    if (tag?.trim()) await api.setAccountTag(selected, tag.trim(), true).catch(notify.error)
  }
  const none = selected.length === 0
  const noPassword = !selectedAccounts.some((a) => a.hasPassword)
  // the accounts' frame rate limit when they all have the same one
  const sameCpu = selectedAccounts.length > 0 && selectedAccounts.every((a) => a.cpuLimit === selectedAccounts[0].cpuLimit)
  const cpu = sameCpu ? selectedAccounts[0].cpuLimit : undefined
  const setCpu = (limit: number | null) => api.setCpuLimit(selected, limit).catch(notify.error)
  const customCpu = async () => {
    const text = await dialogs.prompt({ title: 'CPU limit', label: 'Percent of the whole PC, as Task Manager counts it, from 1 to 99. It applies right away to the account\'s Roblox window.', defaultValue: cpu ? String(cpu) : '10' })
    if (text === null) return
    const n = Number(text.replace('%', '').trim())
    if (!Number.isInteger(n) || n < 1 || n > 99) return notify.error(t('The CPU limit goes from {0}% to {1}%', 1, 99))
    await setCpu(n)
  }

  // the accounts' color when they all have the same one
  const accountsColor = selectedAccounts.length && selectedAccounts.every((a) => a.color === selectedAccounts[0].color) ? selectedAccounts[0].color : null

  if (group) {
    return (
      <ContextMenuContent className="w-56">
        <ContextMenuLabel className="truncate" style={group.color ? { color: group.color } : undefined}>{group.title}</ContextMenuLabel>
        <ContextMenuSub>
          <ContextMenuSubTrigger><Palette /> {t('Group color')}</ContextMenuSubTrigger>
          <ContextMenuSubContent>
            <ColorItems
              current={group.color}
              onPick={(color) => api.setGroupColor(group.key, color).catch(notify.error)}
              onCustom={() => onCustomColor({ kind: 'group', key: group.key, current: group.color })}
            />
          </ContextMenuSubContent>
        </ContextMenuSub>
        <ContextMenuItem
          onClick={async () => {
            // the full name, with the sorting number ("1Main"), so the order can be kept or changed
            const name = await dialogs.prompt({ title: 'Rename group', label: 'A number in front sorts the groups (1Main, 2Alts), it is hidden in the list', defaultValue: group.key })
            if (name && name !== group.key) await api.renameGroup(group.key, name).catch(notify.error)
          }}
        >
          <PenLine /> {t('Rename group…')}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={actions.sortAlphabetically}><ArrowDownAZ /> {t('Sort Alphabetically')}</ContextMenuItem>
        <ContextMenuItem onClick={toggleGroups}><FolderInput /> {t(showGroups ? 'Hide groups' : 'Show groups')}</ContextMenuItem>
      </ContextMenuContent>
    )
  }

  return (
    <ContextMenuContent className="w-60">
      <ContextMenuItem disabled={none} onClick={actions.openHome}><House /> {t('Open Roblox home')}</ContextMenuItem>
      <ContextMenuItem onClick={actions.addAccount}><UserPlus /> {t('Add Account')}</ContextMenuItem>
      <ContextMenuItem disabled={none} onClick={actions.remove} className="text-destructive focus:text-destructive"><Trash2 /> {t('Remove Account')}</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={none}><ClipboardCopy /> {t('Copy')}</ContextMenuSubTrigger>
        <ContextMenuSubContent>
          <ContextMenuItem onClick={() => actions.copy('username')}>{t('Copy Username')}</ContextMenuItem>
          <ContextMenuItem disabled={noPassword} onClick={() => actions.copy('password')}>{t('Copy Password')}</ContextMenuItem>
          <ContextMenuItem disabled={noPassword} onClick={() => actions.copy('combo')}>{t('Copy User:Pass Combo')}</ContextMenuItem>
          <ContextMenuItem onClick={() => actions.copy('profile')}>{t('Copy Profile')}</ContextMenuItem>
          <ContextMenuItem onClick={() => actions.copy('userId')}>{t('Copy UserId')}</ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuSub>
        <ContextMenuSubTrigger><FolderInput /> {t('Groups')}</ContextMenuSubTrigger>
        <ContextMenuSubContent>
          <ContextMenuItem disabled={none} onClick={actions.moveToGroup}>{t('Move Account To')}</ContextMenuItem>
          <ContextMenuItem disabled={none} onClick={() => actions.copy('group')}>{t('Copy Group')}</ContextMenuItem>
          <ContextMenuItem onClick={toggleGroups}>{t(showGroups ? 'Hide groups' : 'Show groups')}</ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={none}><Gauge /> {t('CPU limit')}</ContextMenuSubTrigger>
        <ContextMenuSubContent className="w-52">
          <ContextMenuItem onClick={() => setCpu(null)}>
            {t('No limit')}
            {cpu === null && <Check className="ml-auto" />}
          </ContextMenuItem>
          <ContextMenuSeparator />
          {CPU_CHOICES.map((n) => (
            <ContextMenuItem key={n} onClick={() => setCpu(n)}>
              {n}%
              {cpu === n && <Check className="ml-auto" />}
            </ContextMenuItem>
          ))}
          <ContextMenuSeparator />
          <ContextMenuItem onClick={customCpu}>
            {t('Custom…')}
            {cpu != null && !CPU_CHOICES.includes(cpu) && <span className="ml-auto text-xs text-muted-foreground">{cpu}%</span>}
          </ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuItem disabled={none} onClick={() => api.setPinned(selected, !allPinned).catch(notify.error)}>
        {allPinned ? <PinOff /> : <Pin />} {t(allPinned ? 'Unpin' : 'Pin to the top')}
      </ContextMenuItem>
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={none}><Tag /> {t('Tags')}</ContextMenuSubTrigger>
        <ContextMenuSubContent className="w-52">
          {allTags.map((tag) => {
            const on = selectedAccounts.every((a) => a.tags.includes(tag))
            return (
              <ContextMenuItem key={tag} onClick={() => api.setAccountTag(selected, tag, !on).catch(notify.error)}>
                <span className="size-2.5 rounded-full" style={{ background: tagColor(tag) }} />
                {tag}
                {on && <Check className="ml-auto" />}
              </ContextMenuItem>
            )
          })}
          {allTags.length > 0 && <ContextMenuSeparator />}
          <ContextMenuItem onClick={newTag}>{t('New tag…')}</ContextMenuItem>
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={none}><Palette /> {t('Color')}</ContextMenuSubTrigger>
        <ContextMenuSubContent>
          <ColorItems
            current={accountsColor}
            onPick={(color) => api.setAccountColor(selected, color).catch(notify.error)}
            onCustom={() => onCustomColor({ kind: 'accounts', names: [...selected], current: accountsColor })}
          />
        </ContextMenuSubContent>
      </ContextMenuSub>
      <ContextMenuItem onClick={actions.sortAlphabetically}><ArrowDownAZ /> {t('Sort Alphabetically')}</ContextMenuItem>
      <ContextMenuItem disabled={selected.length !== 1} onClick={actions.quickLogIn}><LogIn /> {t('Quick Log In')}</ContextMenuItem>
      <ContextMenuItem disabled={none} onClick={actions.dumpDetails}><FileText /> {t('Dump Details')}</ContextMenuItem>
      <ContextMenuItem
        onClick={() =>
          dialogs.alert(
            'Groups',
            'Groups can be sorted by naming them a number then whatever you want.\nFor example: You can put Group Apple on top by naming it \'001 Apple\' or \'1Apple\'.\nThe numbers will be hidden from the name but will be correctly sorted depending on the number.\nAccounts can also be dragged into groups.',
          )
        }
      >
        <Info /> {t('Help')}
      </ContextMenuItem>

      {devMode && (
        <>
          <ContextMenuSeparator />
          <ContextMenuItem disabled={selected.length !== 1} onClick={actions.viewFields}><Code /> {t('View Fields')}</ContextMenuItem>
          <ContextMenuItem disabled={none} onClick={() => actions.copy('ticket')}><KeyRound /> {t('Get Authentication Ticket')}</ContextMenuItem>
          <ContextMenuItem disabled={none} onClick={() => actions.copy('token')}>{t('Copy Security Token')}</ContextMenuItem>
          <ContextMenuItem disabled={selected.length !== 1} onClick={() => actions.copy('playerLink')}>{t('Copy rbx-player Link')}</ContextMenuItem>
          <ContextMenuItem disabled={selected.length !== 1} onClick={() => actions.copy('appLink')}>{t('Copy App Link')}</ContextMenuItem>
        </>
      )}
    </ContextMenuContent>
  )
}
