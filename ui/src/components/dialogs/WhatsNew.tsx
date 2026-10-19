/** "What's new" before updating: the notes of every version since this one (several when the app is far behind),
 *  then the download or the restart. whatsNew() resolves true when the person goes ahead. */

import { useEffect, useState, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { api } from '@/lib/api'
import type { ReleaseNotes } from '@/lib/types'
import { useT } from '@/lib/i18n'

/** before a download or a restart, or just reading the latest versions' notes */
type Step = 'download' | 'restart' | 'read'

let show: ((step: Step) => Promise<boolean>) | null = null

/** Opens the notes; without the dialog mounted it just goes ahead */
export const whatsNew = (step: Step) => (show ? show(step) : Promise.resolve(true))

/** **bold**, `code` and [links](…) (as their text) inside a line */
function inline(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g).map((part, i) => {
    if (part.startsWith('**') && part.endsWith('**')) return <strong key={i} className="font-medium text-foreground">{part.slice(2, -2)}</strong>
    if (part.startsWith('`') && part.endsWith('`')) return <code key={i} className="rounded bg-muted px-1 font-mono text-xs">{part.slice(1, -1)}</code>
    const link = /^\[([^\]]+)\]\([^)]+\)$/.exec(part)
    return link ? link[1] : part
  })
}

/** The few bits of markdown the release notes use, as plain elements (nothing from GitHub becomes HTML) */
function Markdown({ text }: { text: string }) {
  const blocks: ReactNode[] = []
  let items: ReactNode[] = []
  const endList = () => {
    if (items.length) blocks.push(<ul key={blocks.length} className="list-disc space-y-1 pl-5">{items}</ul>)
    items = []
  }
  let code = false
  for (const raw of text.split('\n')) {
    const line = raw.trim()
    if (line.startsWith('```')) {
      code = !code
      continue
    }
    if (code) continue
    const item = /^[-*] (.*)$/.exec(line)
    if (item) {
      items.push(<li key={items.length}>{inline(item[1])}</li>)
      continue
    }
    endList()
    const heading = /^#{1,6} (.*)$/.exec(line)
    if (heading) blocks.push(<p key={blocks.length} className="pt-1 font-medium text-foreground">{inline(heading[1])}</p>)
    else if (line) blocks.push(<p key={blocks.length}>{inline(line)}</p>)
  }
  endList()
  return <div className="space-y-2 text-sm text-muted-foreground">{blocks}</div>
}

export function WhatsNewHost() {
  const t = useT()
  const [step, setStep] = useState<Step | null>(null)
  const [notes, setNotes] = useState<ReleaseNotes[] | null>(null)
  const [failed, setFailed] = useState(false)
  const [resolve, setResolve] = useState<((ok: boolean) => void) | null>(null)

  useEffect(() => {
    show = (s) =>
      new Promise<boolean>((done) => {
        setStep(s)
        setNotes(null)
        setFailed(false)
        setResolve(() => done)
        api.updateNotes(s === 'read').then(setNotes, () => setFailed(true))
      })
    return () => {
      show = null
    }
  }, [])

  const finish = (ok: boolean) => {
    resolve?.(ok)
    setResolve(null)
    setStep(null)
  }

  return (
    <Dialog open={!!step} onOpenChange={(open) => !open && finish(false)}>
      <DialogContent className="sm:max-w-lg" data-testid="whats-new">
        <DialogHeader>
          <DialogTitle>{t("What's new")}</DialogTitle>
          <DialogDescription>
            {step === 'read' ? t('The latest versions') : notes && notes.length > 1 ? t('{0} new versions since yours', notes.length) : t('Changes in the new version')}
            {step === 'restart' && <> {t('Anti-AFK stops while the app restarts.')}</>}
          </DialogDescription>
        </DialogHeader>
        <div className="max-h-[55vh] overflow-y-auto pr-3">
          {failed && <p className="text-sm text-muted-foreground">{t("Couldn't get the release notes, you can still update.")}</p>}
          {!notes && !failed && <p className="text-sm text-muted-foreground">{t('Loading…')}</p>}
          <div className="space-y-5">
            {notes?.map((n) => (
              <section key={n.version} className="space-y-2">
                <h3 className="text-sm font-semibold">
                  {t('Version {0}', n.version)} <span className="font-normal text-muted-foreground">{n.date}</span>
                </h3>
                <Markdown text={n.text} />
              </section>
            ))}
          </div>
        </div>
        <DialogFooter>
          {step === 'read' ? (
            <Button onClick={() => finish(true)}>{t('Close')}</Button>
          ) : (
            <>
              <Button variant="outline" onClick={() => finish(false)}>{t('Later')}</Button>
              <Button onClick={() => finish(true)}>{step === 'restart' ? t('Restart to update') : t('Download update')}</Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
