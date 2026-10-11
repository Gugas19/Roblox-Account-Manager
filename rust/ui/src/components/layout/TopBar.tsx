/** The bar above everything: add/remove/open browser on the left, language, theme and settings on the right */

import { ChevronDown, Command, Download, Globe, MessageCircle, Palette, Plus, Settings, Trash2, UserRoundPlus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Progress } from '@/components/ui/progress'
import { Switch } from '@/components/ui/switch'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { AntiAfkPill } from '@/components/antiafk/AntiAfkPill'
import { LanguageMenu } from '@/components/layout/LanguageMenu'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
  const t = useT()
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={t(label)} onClick={onClick}>
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

  return (
    <header className="flex items-center gap-2 px-4 py-3">
      <div className="flex">
        <Button className="rounded-r-none" onClick={actions.addAccount} disabled={download !== null}>
          <UserRoundPlus /> {t('Add Account')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button className="rounded-l-none border-l border-primary-foreground/20 px-2" aria-label={t('More ways to add accounts')}>
              <ChevronDown />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-56">
            <DropdownMenuItem onClick={actions.addAccount}><Globe /> {t('Log in with the browser')}</DropdownMenuItem>
            <DropdownMenuItem onClick={actions.importCookies}><Download /> {t('Cookie(s)')}</DropdownMenuItem>
            <DropdownMenuItem onClick={actions.addUserPass}><Plus /> {t('User:Pass')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={actions.customBrowser}>{t('Custom URL & Javascript')}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <Button variant="secondary" disabled={none} onClick={actions.remove}>
        <Trash2 /> {t('Remove')}
      </Button>

      <div className="flex">
        <Button variant="secondary" className="rounded-r-none" disabled={none} onClick={() => actions.openBrowser('home')}>
          <Globe /> {t('Open Browser')}
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="secondary" className="rounded-l-none border-l border-border px-2" disabled={none} aria-label={t('More browser options')}>
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

      <label className="ml-2 flex cursor-pointer items-center gap-2 text-sm text-muted-foreground select-none">
        <Switch checked={hideUsernames} onCheckedChange={setHideUsernames} />
        {t('Hide Usernames')}
      </label>

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
        <AntiAfkPill />
        <Button variant="ghost" size="sm" className="gap-1 text-muted-foreground" onClick={onPalette}>
          <Command className="size-3.5" /> <kbd className="text-xs">Ctrl K</kbd>
        </Button>
        <IconButton label="Join Discord" onClick={() => api.open('discord')}>
          <MessageCircle />
        </IconButton>
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
