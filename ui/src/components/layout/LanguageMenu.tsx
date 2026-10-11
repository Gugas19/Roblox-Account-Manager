/** The flag button: changes the whole interface language right away */

import { Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Flag } from '@/components/common/Flag'
import { api } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function LanguageMenu() {
  const { language, setTable, t } = useI18n()
  const { boot } = useStore()
  const current = boot.languages.find((l) => l.code === language) ?? boot.languages[0]

  const change = async (code: string) => {
    const result = await api.setLanguage(code)
    setTable(result.language, result.table)
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="gap-2" aria-label={t('Language')}>
          <Flag code={current.flag} />
          <span className="text-xs font-semibold uppercase">{current.code.slice(0, 2)}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-52">
        {boot.languages.map((l) => (
          <DropdownMenuItem key={l.code} onClick={() => change(l.code)}>
            <Flag code={l.flag} />
            <span className={l.code === language ? 'font-semibold' : ''}>{l.name}</span>
            {l.code === language && <Check className="ml-auto" />}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
