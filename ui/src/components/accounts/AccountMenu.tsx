/** Right click menu of the account list (same items as the C# version, developer ones only in developer mode) */

import { ArrowDownAZ, Check, ClipboardCopy, Code, FileText, FolderInput, Gauge, Info, KeyRound, LogIn, Palette, PenLine, Pin, Trash2, UserPlus } from 'lucide-react'
import { ContextMenuContent, ContextMenuItem, ContextMenuLabel, ContextMenuSeparator, ContextMenuSub, ContextMenuSubContent, ContextMenuSubTrigger } from '@/components/ui/context-menu'
import { ColorItems } from '@/components/accounts/ColorItems'
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
const FPS_CHOICES = [15, 30, 45, 60, 90, 120, 144, 240]

export function AccountMenu({ showGroups, toggleGroups, group, onCustomColor }: Props) {
  const t = useT()
  const notify = notifier(t)
  const actions = useActions()
  const dialogs = useDialogs()
  const { devMode, selected, selectedAccounts } = useStore()
  const none = selected.length === 0
  const noPassword = !selectedAccounts.some((a) => a.hasPassword)
  const hasSavedGame = selectedAccounts.some((a) => a.savedPlaceId || a.savedJobId)
  // the accounts' frame rate limit when they all have the same one
  const sameFps = selectedAccounts.length > 0 && selectedAccounts.every((a) => a.fpsLimit === selectedAccounts[0].fpsLimit)
  const fps = sameFps ? selectedAccounts[0].fpsLimit : undefined
  const setFps = (limit: number | null) => api.setFpsLimit(selected, limit).catch(notify.error)
  const customFps = async () => {
    const text = await dialogs.prompt({ title: 'FPS limit', label: 'Frames per second, from 5 to 1000. Applied the next time the account launches.', defaultValue: fps ? String(fps) : '60' })
    if (text === null) return
    const n = Number(text)
    if (!Number.isInteger(n) || n < 5 || n > 1000) return notify.error(t('The frame rate limit goes from {0} to {1}', 5, 1000))
    await setFps(n)
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
        <ContextMenuSubTrigger disabled={none}><Gauge /> {t('FPS limit')}</ContextMenuSubTrigger>
        <ContextMenuSubContent className="w-52">
          <ContextMenuItem onClick={() => setFps(null)}>
            {t('Like in the settings')}
            {fps === null && <Check className="ml-auto" />}
          </ContextMenuItem>
          <ContextMenuSeparator />
          {FPS_CHOICES.map((n) => (
            <ContextMenuItem key={n} onClick={() => setFps(n)}>
              {t('{0} FPS', n)}
              {fps === n && <Check className="ml-auto" />}
            </ContextMenuItem>
          ))}
          <ContextMenuSeparator />
          <ContextMenuItem onClick={customFps}>
            {t('Custom…')}
            {fps != null && !FPS_CHOICES.includes(fps) && <span className="ml-auto text-xs text-muted-foreground">{t('{0} FPS', fps)}</span>}
          </ContextMenuItem>
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
      {/* a game (and server) the account always joins, and fills in the panel when it's selected */}
      <ContextMenuSub>
        <ContextMenuSubTrigger disabled={none}><Pin /> {t('Saved game')}</ContextMenuSubTrigger>
        <ContextMenuSubContent className="w-64">
          <ContextMenuItem onClick={actions.saveToAccount}>{t('Always join the game in the panel')}</ContextMenuItem>
          <ContextMenuItem disabled={!hasSavedGame} onClick={() => api.savePlace(selected, '', '').then(() => notify.success('Removed'), notify.error)}>{t('Forget the saved game')}</ContextMenuItem>
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
