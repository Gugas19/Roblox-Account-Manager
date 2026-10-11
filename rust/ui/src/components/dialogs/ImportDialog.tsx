/** Import accounts by their .ROBLOSECURITY cookies, with a result per line */

import { CheckCircle2, Loader2, XCircle } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Textarea } from '@/components/ui/textarea'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function ImportDialog() {
  const t = useT()
  const { panel, setPanel } = useStore()
  const [text, setText] = useState('')
  const [busy, setBusy] = useState(false)
  const [results, setResults] = useState<{ text: string; ok: boolean }[]>([])

  const run = async () => {
    setBusy(true)
    setResults(await api.importCookies(text))
    setText('')
    setBusy(false)
  }

  return (
    <Dialog open={panel === 'import'} onOpenChange={(open) => { if (!open) { setPanel(null); setResults([]) } }}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t('Import Accounts')}</DialogTitle>
          <DialogDescription>{t('Import Accounts by ROBLOSECURITY, separated by new lines')}</DialogDescription>
        </DialogHeader>
        <Textarea className="min-h-40 font-mono text-xs" value={text} onChange={(e) => setText(e.target.value)} placeholder="_|WARNING:-DO-NOT-SHARE-THIS…" />
        {results.length > 0 && (
          <div className="grid max-h-40 gap-1 overflow-y-auto text-sm">
            {results.map((r, i) => (
              <div key={i} className="flex items-center gap-2">
                {r.ok ? <CheckCircle2 className="size-4 text-emerald-500" /> : <XCircle className="size-4 text-destructive" />}
                {r.ok ? t('Added {0}', r.text) : t('Failed to add {0}', r.text)}
              </div>
            ))}
          </div>
        )}
        <DialogFooter>
          <Button disabled={!text.trim() || busy} onClick={run}>
            {busy && <Loader2 className="animate-spin" />} {t('Import')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
