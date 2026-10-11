/** A setting with a short explanation, bound to RAMSettings.ini (section/key) */

import type { ReactNode } from 'react'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function SettingRow({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  const t = useT()
  return (
    <div className="flex items-center justify-between gap-6 py-2.5">
      <div className="min-w-0">
        <div className="text-sm font-medium">{t(title)}</div>
        {hint && <div className="text-xs text-muted-foreground">{t(hint)}</div>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

export function Toggle({ section, name, title, hint, disabled, onChange }: { section: string; name: string; title: string; hint?: string; disabled?: boolean; onChange?: (on: boolean) => void }) {
  const { flag, updateSetting } = useStore()
  return (
    <SettingRow title={title} hint={hint}>
      <Switch
        checked={flag(section, name)}
        disabled={disabled}
        onCheckedChange={(on) => {
          void updateSetting(section, name, on ? 'true' : 'false')
          onChange?.(on)
        }}
      />
    </SettingRow>
  )
}

export function NumberSetting({ section, name, title, hint, min, max, fallback }: { section: string; name: string; title: string; hint?: string; min: number; max: number; fallback: number }) {
  const { setting, updateSetting } = useStore()
  const value = Number(setting(section, name) || fallback)
  return (
    <SettingRow title={title} hint={hint}>
      <Input
        type="number"
        className="w-24"
        min={min}
        max={max}
        defaultValue={value}
        key={value}
        onBlur={(e) => {
          const n = Math.min(max, Math.max(min, Number(e.target.value) || fallback))
          e.target.value = String(n)
          if (n !== value) void updateSetting(section, name, String(n))
        }}
        onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
      />
    </SettingRow>
  )
}

export function TextSetting({ section, name, title, hint, placeholder, password, width = 'w-56' }: { section: string; name: string; title: string; hint?: string; placeholder?: string; password?: boolean; width?: string }) {
  const { setting, updateSetting } = useStore()
  const value = setting(section, name)
  return (
    <SettingRow title={title} hint={hint}>
      <Input
        className={width}
        type={password ? 'password' : 'text'}
        defaultValue={value}
        key={value}
        placeholder={placeholder}
        onBlur={(e) => e.target.value !== value && void updateSetting(section, name, e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
      />
    </SettingRow>
  )
}
