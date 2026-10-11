/** Ctrl+K: find any account or action and run it */

import { ArrowDownAZ, Cookie, Globe, Languages, Palette, Play, Settings, ShieldAlert, Trash2, UserRoundPlus, Users, Wrench } from 'lucide-react'
import { Command, CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator } from '@/components/ui/command'
import { useActions } from '@/hooks/useActions'
import { api } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function CommandPalette({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t, setTable } = useI18n()
  const actions = useActions()
  const { accounts, select, boot, selected } = useStore()

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
          <CommandItem onSelect={run(() => actions.openBrowser('home'))} disabled={!selected.length}><Globe /> {t('Open Browser')}</CommandItem>
          <CommandItem onSelect={run(actions.remove)} disabled={!selected.length}><Trash2 /> {t('Remove')}</CommandItem>
          <CommandItem onSelect={run(actions.sortAlphabetically)}><ArrowDownAZ /> {t('Sort Alphabetically')}</CommandItem>
          <CommandItem onSelect={run(() => actions.open('antiafk'))}><ShieldAlert /> {t('Anti-AFK')}</CommandItem>
          <CommandItem onSelect={run(() => actions.open('utilities'))}><Wrench /> {t('Utilities')}</CommandItem>
          <CommandItem onSelect={run(() => actions.open('settings'))}><Settings /> {t('Settings')}</CommandItem>
          <CommandItem onSelect={run(() => actions.open('theme'))}><Palette /> {t('Theme Editor')}</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={t('Accounts')}>
          {accounts.map((a) => (
            <CommandItem key={a.username} value={`${a.username} ${a.alias} ${a.groupTitle}`} onSelect={run(() => select(a.username))}>
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
