/** confirm(), prompt() and alert() as promises, shown with shadcn dialogs (they replace the C# message boxes) */

import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from 'react'
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useT } from '@/lib/i18n'

interface ConfirmOptions {
  title: string
  description?: string
  confirm?: string
  cancel?: string
  destructive?: boolean
  /** only an OK button */
  alert?: boolean
}

interface PromptOptions {
  title: string
  label?: string
  defaultValue?: string
  placeholder?: string
  multiline?: boolean
  password?: boolean
  confirm?: string
}

interface Dialogs {
  confirm: (options: ConfirmOptions) => Promise<boolean>
  alert: (title: string, description?: string) => Promise<void>
  prompt: (options: PromptOptions) => Promise<string | null>
}

const Context = createContext<Dialogs>(null as unknown as Dialogs)

export const useDialogs = () => useContext(Context)

export function DialogHost({ children }: { children: ReactNode }) {
  const t = useT()
  const [confirmState, setConfirm] = useState<ConfirmOptions | null>(null)
  const [promptState, setPrompt] = useState<PromptOptions | null>(null)
  const [value, setValue] = useState('')
  const resolver = useRef<((v: unknown) => void) | null>(null)

  const finish = (result: unknown) => {
    resolver.current?.(result)
    resolver.current = null
    setConfirm(null)
    setPrompt(null)
  }

  const confirm = useCallback((options: ConfirmOptions) => new Promise<boolean>((resolve) => {
    resolver.current = resolve as (v: unknown) => void
    setConfirm(options)
  }), [])

  const alert = useCallback((title: string, description?: string) => confirm({ title, description, alert: true }).then(() => undefined), [confirm])

  const prompt = useCallback((options: PromptOptions) => new Promise<string | null>((resolve) => {
    resolver.current = resolve as (v: unknown) => void
    setValue(options.defaultValue ?? '')
    setPrompt(options)
  }), [])

  return (
    <Context.Provider value={{ confirm, alert, prompt }}>
      {children}

      <AlertDialog open={!!confirmState} onOpenChange={(open) => !open && finish(false)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t(confirmState?.title ?? '')}</AlertDialogTitle>
            {confirmState?.description && <AlertDialogDescription className="whitespace-pre-line break-words">{t(confirmState.description)}</AlertDialogDescription>}
          </AlertDialogHeader>
          <AlertDialogFooter>
            {!confirmState?.alert && <AlertDialogCancel onClick={() => finish(false)}>{t(confirmState?.cancel ?? 'No')}</AlertDialogCancel>}
            <AlertDialogAction className={confirmState?.destructive ? 'bg-destructive text-white hover:bg-destructive/90' : ''} onClick={() => finish(true)}>
              {t(confirmState?.confirm ?? (confirmState?.alert ? 'OK' : 'Yes'))}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <Dialog open={!!promptState} onOpenChange={(open) => !open && finish(null)}>
        <DialogContent className={promptState?.multiline ? 'sm:max-w-xl' : 'sm:max-w-md'}>
          <form
            className="grid gap-4"
            onSubmit={(e) => {
              e.preventDefault()
              finish(value)
            }}
          >
            <DialogHeader>
              <DialogTitle>{t(promptState?.title ?? '')}</DialogTitle>
              {promptState?.label && <DialogDescription className="whitespace-pre-line">{t(promptState.label)}</DialogDescription>}
            </DialogHeader>
            {promptState?.multiline ? (
              <Textarea autoFocus className="min-h-48 font-mono text-xs" value={value} placeholder={promptState?.placeholder} onChange={(e) => setValue(e.target.value)} />
            ) : (
              <Input autoFocus type={promptState?.password ? 'password' : 'text'} value={value} placeholder={promptState?.placeholder} onChange={(e) => setValue(e.target.value)} />
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => finish(null)}>{t('Cancel')}</Button>
              <Button type="submit">{t(promptState?.confirm ?? 'OK')}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </Context.Provider>
  )
}
