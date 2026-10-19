/** Theme editor: presets and every color of RAMTheme.ini, applied as you pick */

import { RotateCcw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { SettingRow } from '@/components/common/SettingRow'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

const COLORS: [string, string][] = [
  ['AccentColor', 'Accent'],
  ['FormsBG', 'Background'],
  ['FormsFG', 'Text'],
  ['AccountsBG', 'Cards and account list'],
  ['AccountsFG', 'Account list text'],
  ['ButtonsBG', 'Buttons'],
  ['ButtonsFG', 'Button text'],
  ['TextBoxesBG', 'Text Boxes'],
]

const PRESETS: { name: string; colors: Record<string, string> }[] = [
  { name: 'Dark', colors: { FormsBG: '#14161C', FormsFG: '#E6E8EE', AccountsBG: '#1A1C24', AccountsFG: '#E6E8EE', ButtonsBG: '#262935', ButtonsFG: '#E6E8EE', TextBoxesBG: '#1F2230', AccentColor: '#4F7CFF' } },
  { name: 'Midnight', colors: { FormsBG: '#0B0D12', FormsFG: '#E2E8F0', AccountsBG: '#11141B', AccountsFG: '#E2E8F0', ButtonsBG: '#1C2130', ButtonsFG: '#E2E8F0', TextBoxesBG: '#161A24', AccentColor: '#8B5CF6' } },
  { name: 'Forest', colors: { FormsBG: '#111714', FormsFG: '#E3EDE7', AccountsBG: '#16201B', AccountsFG: '#E3EDE7', ButtonsBG: '#22302A', ButtonsFG: '#E3EDE7', TextBoxesBG: '#1B2621', AccentColor: '#22C55E' } },
  { name: 'Light', colors: { FormsBG: '#F4F5F8', FormsFG: '#16181D', AccountsBG: '#FFFFFF', AccountsFG: '#16181D', ButtonsBG: '#E8EAF0', ButtonsFG: '#16181D', TextBoxesBG: '#FFFFFF', AccentColor: '#2563EB' } },
]

/** Settings > Appearance: ready-made themes, each color, the list's headers */
export function ThemeSection() {
  const t = useT()
  const { theme, updateTheme, resetTheme } = useStore()

  const apply = async (colors: Record<string, string>) => {
    for (const [key, value] of Object.entries(colors)) await updateTheme(key, value)
  }

  return (
    <div className="grid gap-4 py-2" data-testid="theme-editor">

        <div className="grid grid-cols-4 gap-2">
          {PRESETS.map((p) => (
            <button key={p.name} onClick={() => apply(p.colors)} className="group grid gap-1.5 rounded-lg border p-2 text-xs transition-colors hover:border-primary">
              <div className="flex h-10 overflow-hidden rounded-md" style={{ background: p.colors.FormsBG }}>
                <div className="m-1.5 flex-1 rounded" style={{ background: p.colors.AccountsBG }} />
                <div className="m-1.5 ml-0 w-3 rounded" style={{ background: p.colors.AccentColor }} />
              </div>
              {t(p.name)}
            </button>
          ))}
        </div>

        <div className="divide-y">
          {COLORS.map(([key, label]) => (
            <SettingRow key={key} title={label}>
              <label className="flex cursor-pointer items-center gap-2 font-mono text-xs text-muted-foreground">
                {theme[key]}
                <input type="color" className="size-8 cursor-pointer rounded-md border bg-transparent" value={theme[key] ?? '#000000'} onChange={(e) => updateTheme(key, e.target.value.toUpperCase())} />
              </label>
            </SettingRow>
          ))}
          <SettingRow title="Show Headers" hint="Column names above the account list.">
            <Switch checked={(theme.ShowHeaders ?? 'True').toLowerCase() !== 'false'} onCheckedChange={(on) => updateTheme('ShowHeaders', on ? 'True' : 'False')} />
          </SettingRow>
        </div>

        <Button variant="outline" className="justify-self-start" onClick={resetTheme}>
          <RotateCcw /> {t('Reset to the default theme')}
        </Button>
    </div>
  )
}
