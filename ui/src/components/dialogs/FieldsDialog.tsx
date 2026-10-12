/** Developer mode: the custom fields saved on an account (used by scripts through the web API) */

import { Plus, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

export function FieldsDialog() {
  const t = useT()
  const notify = notifier(t)
  const { panel, setPanel, selected } = useStore()
  const name = selected[0]
  const [fields, setFields] = useState<Record<string, string>>({})
  const [key, setKey] = useState('')
  const [value, setValue] = useState('')

  useEffect(() => {
    if (panel === 'fields' && name) api.accountFields(name).then(setFields)
  }, [panel, name])

  const save = async (k: string, v: string) => {
    try {
      await api.setField(name, k, v)
      setFields(await api.accountFields(name))
    } catch (e) {
      notify.error(e)
    }
  }

  return (
    <Dialog open={panel === 'fields'} onOpenChange={(open) => !open && setPanel(null)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t('Account Fields')}</DialogTitle>
          <DialogDescription>{name}</DialogDescription>
        </DialogHeader>
        <div className="grid max-h-80 gap-2 overflow-y-auto">
          {Object.entries(fields).map(([k, v]) => (
            <div key={k} className="flex items-center gap-2">
              <span className="w-40 truncate font-mono text-xs" title={k}>{k}</span>
              <Input defaultValue={v} onBlur={(e) => e.target.value !== v && save(k, e.target.value)} />
              <Button size="icon" variant="ghost" aria-label={t('Remove')} onClick={() => save(k, '')}><Trash2 /></Button>
            </div>
          ))}
          {Object.keys(fields).length === 0 && <p className="text-sm text-muted-foreground">{t('No fields yet')}</p>}
        </div>
        <form className="flex gap-2" onSubmit={async (e) => { e.preventDefault(); if (key && value) { await save(key, value); setKey(''); setValue('') } }}>
          <Input value={key} onChange={(e) => setKey(e.target.value)} placeholder={t('Field')} className="w-40" />
          <Input value={value} onChange={(e) => setValue(e.target.value)} placeholder={t('Value')} />
          <Button type="submit" variant="secondary" disabled={!key.trim() || !value}><Plus /> {t('Add')}</Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
