/** The bar above everything: add/remove/open browser on the left, language, theme and settings on the right.
 *  The less used buttons are icons with a tooltip, so the bar fits in every language with the update and Anti-AFK
 *  buttons showing. */

import { ChevronDown, Command, Download, Eye, EyeOff, Globe, Palette, Plus, Settings, Trash2, UserRoundPlus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Progress } from '@/components/ui/progress'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { AntiAfkPill } from '@/components/antiafk/AntiAfkPill'
import { UpdatePill } from './UpdatePill'
import { LanguageMenu } from '@/components/layout/LanguageMenu'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

function IconButton({ label, onClick, children, variant = 'ghost', disabled, pressed }: { label: string; onClick: () => void; children: React.ReactNode; variant?: 'ghost' | 'secondary'; disabled?: boolean; pressed?: boolean }) {
  const t = useT()
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant={variant} size="icon" aria-label={t(label)} aria-pressed={pressed} disabled={disabled} onClick={onClick}>
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{t(label)}</TooltipContent>
    </Tooltip>
  )
}

export function TopBar({ onPalette }: { onPalette: () => void }) {
  const t = useT()
  const actions = useActions()
  const { selected, download, hideUsernames, setHideUsernames } = useStore()
  const none = selected.length === 0
  // the browser is being downloaded, everything that opens it waits
  const downloading = download !== null

  return (
    <header className="flex items-center gap-2 px-4 py-3 whitespace-nowrap">
      <div className="flex">
        <Button className="rounded-r-none" onClick={actions.addAccount} disabled={downloading}>
          <UserRoundPlus /> {t('Add Account')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button className="rounded-l-none border-l border-primary-foreground/20 px-2" aria-label={t('More ways to add accounts')}>
              <ChevronDown />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-56">
            <DropdownMenuItem disabled={downloading} onClick={actions.addAccount}><Globe /> {t('Log in with the browser')}</DropdownMenuItem>
            <DropdownMenuItem onClick={actions.importCookies}><Download /> {t('Cookie(s)')}</DropdownMenuItem>
            <DropdownMenuItem disabled={downloading} onClick={actions.addUserPass}><Plus /> {t('User:Pass')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem disabled={downloading} onClick={actions.customBrowser}>{t('Custom URL & Javascript')}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <IconButton label="Remove" variant="secondary" disabled={none} onClick={actions.remove}>
        <Trash2 />
      </IconButton>

      <div className="flex">
        <Button variant="secondary" className="rounded-r-none" disabled={none || downloading} onClick={() => actions.openBrowser('home')}>
          <Globe /> {t('Open Browser')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="secondary" className="rounded-l-none border-l border-border px-2" disabled={none || downloading} aria-label={t('More browser options')}>
              <ChevronDown />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            <DropdownMenuLabel>{t('Open Browser')}</DropdownMenuLabel>
            <DropdownMenuItem onClick={() => actions.openBrowser('url')}>{t('URL')}</DropdownMenuItem>
            <DropdownMenuItem onClick={() => actions.openBrowser('script')}>{t('URL + Javascript')}</DropdownMenuItem>
            <DropdownMenuItem onClick={() => actions.openBrowser('group')}>{t('Join Group')}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <IconButton label="Hide Usernames" pressed={hideUsernames} onClick={() => setHideUsernames(!hideUsernames)}>
        {hideUsernames ? <EyeOff className="text-primary" /> : <Eye />}
      </IconButton>

      <div className="ml-auto flex items-center gap-1">
        {download !== null && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button className="mr-2 flex w-40 flex-col gap-1 text-left text-xs text-muted-foreground" onClick={() => api.open('browser')}>
                {t('Downloading the browser...')}
                <Progress value={download * 100} className="h-1.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>{t('Accounts can be added once it is done')}</TooltipContent>
          </Tooltip>
        )}
        <UpdatePill />
        <AntiAfkPill />
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={t('Command palette')} onClick={onPalette}>
              <Command />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {t('Command palette')} <kbd className="ml-1 text-xs opacity-70">Ctrl K</kbd>
          </TooltipContent>
        </Tooltip>
        <LanguageMenu />
        <IconButton label="Theme Editor" onClick={() => actions.open('theme')}>
          <Palette />
        </IconButton>
        <IconButton label="Settings" onClick={() => actions.open('settings')}>
          <Settings />
        </IconButton>
      </div>
    </header>
  )
}
