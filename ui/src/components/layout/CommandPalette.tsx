/** Ctrl+K: find any account, action or page and go to it */

import { ArrowDownAZ, Cookie, Gamepad2, Globe, House, Languages, Palette, Play, Settings, ShieldCheck, Shirt, Trash2, UserRoundPlus, Users } from 'lucide-react'
import { Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator } from '@/components/ui/command'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function CommandPalette({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t, setTable } = useI18n()
  const actions = useActions()
  const { accounts, select, boot, selected, setPage } = useStore()

  const run = (work: () => unknown) => () => {
    onOpenChange(false)
    void work()
  }

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange} title={t('Command palette')} description={t('Search accounts and actions')}>
      <Command>
      <CommandInput placeholder={t('Search accounts and actions')} />
      <CommandList>
        <CommandEmpty>{t('Nothing found')}</CommandEmpty>
        <CommandGroup heading={t('Actions')}>
          <CommandItem onSelect={run(actions.join)} disabled={!selected.length}><Play /> {t('Join Server')}</CommandItem>
          <CommandItem onSelect={run(actions.addAccount)}><UserRoundPlus /> {t('Add Account')}</CommandItem>
          <CommandItem onSelect={run(actions.importCookies)}><Cookie /> {t('Cookie(s)')}</CommandItem>
          <CommandItem onSelect={run(actions.openHome)} disabled={!selected.length}><House /> {t('Open Roblox home')}</CommandItem>
          <CommandItem onSelect={run(() => actions.openBrowser('home'))} disabled={!selected.length}><Globe /> {t('Open Browser')}</CommandItem>
          <CommandItem onSelect={run(actions.remove)} disabled={!selected.length}><Trash2 /> {t('Remove')}</CommandItem>
          <CommandItem onSelect={run(actions.sortAlphabetically)}><ArrowDownAZ /> {t('Sort Alphabetically')}</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t('Pages')}>
          <CommandItem onSelect={run(() => setPage('accounts'))}><Users /> {t('Accounts')}</CommandItem>
          <CommandItem onSelect={run(() => setPage('games'))}><Gamepad2 /> {t('Games')}</CommandItem>
          <CommandItem onSelect={run(() => setPage('avatar'))}><Shirt /> {t('Avatar')}</CommandItem>
          <CommandItem onSelect={run(() => setPage('antiafk'))}><ShieldCheck /> {t('Anti-AFK')}</CommandItem>
          <CommandItem onSelect={run(() => setPage('settings'))}><Settings /> {t('Settings')}</CommandItem>
          <CommandItem onSelect={run(() => setPage('settings', 'appearance'))}><Palette /> {t('Appearance')}</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t('Accounts')}>
          {accounts.map((a) => (
            <CommandItem key={a.username} value={`${a.username} ${a.alias} ${a.groupTitle}`} onSelect={run(() => { setPage('accounts'); select(a.username) })}>
              <Users /> {a.username}
              {a.alias && <span className="text-muted-foreground">· {a.alias}</span>}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t('Language')}>
          {boot.languages.map((l) => (
            <CommandItem key={l.code} onSelect={run(async () => { const r = await api.setLanguage(l.code); setTable(r.language, r.table) })}>
              <Languages /> {l.name}
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
      </Command>
    </CommandDialog>
  )
}
