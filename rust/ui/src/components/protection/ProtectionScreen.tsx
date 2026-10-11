/** Before the accounts: choose how they're protected (first run), unlock them, or restore the backup */

import { KeyRound, Loader2, LockKeyhole, MonitorSmartphone, ShieldCheck } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import type { LoadState } from '@/lib/types'

function Shell({ icon, title, description, children }: { icon: React.ReactNode; title: string; description: string; children: React.ReactNode }) {
  const t = useT()
  return (
    <div className="grid h-full place-items-center p-6">
      <Card className="w-full max-w-lg">
        <CardHeader className="items-center text-center">
          <div className="mx-auto mb-2 grid size-12 place-items-center rounded-2xl bg-primary/15 text-primary">{icon}</div>
          <CardTitle className="text-xl">{t(title)}</CardTitle>
          <CardDescription>{t(description)}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">{children}</CardContent>
      </Card>
    </div>
  )
}

function Choice({ icon, title, text, recommended, onClick }: { icon: React.ReactNode; title: string; text: string; recommended?: boolean; onClick: () => void }) {
  const t = useT()
  return (
    <button onClick={onClick} className="flex items-start gap-3 rounded-xl border p-4 text-left transition-colors hover:border-primary hover:bg-primary/5">
      <div className="mt-0.5 text-primary">{icon}</div>
      <div>
        <div className="flex items-center gap-2 font-medium">
          {t(title)}
          {recommended && <span className="rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary uppercase">{t('Recommended')}</span>}
        </div>
        <div className="mt-1 text-sm text-muted-foreground">{t(text)}</div>
      </div>
    </button>
  )
}

export function ProtectionScreen({ state, onReady }: { state: LoadState; onReady: () => void }) {
  const t = useT()
  const [step, setStep] = useState<LoadState | { state: 'newPassword' }>(state)
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  const run = async (work: () => Promise<void>) => {
    setBusy(true)
    setError('')
    try {
      await work()
    } catch (e) {
      setError(t(e instanceof Error ? e.message : String(e)))
    } finally {
      setBusy(false)
    }
  }

  if (step.state === 'chooseProtection') {
    return (
      <Shell icon={<ShieldCheck />} title="How should your accounts be protected?" description="Your account cookies are saved in AccountData.json, choose how that file is encrypted. You can change it later in the settings.">
        <Choice recommended icon={<KeyRound />} title="Password protected" text="Encrypted with a password you type when the app starts. The file works on any PC, safe to back up." onClick={() => setStep({ state: 'newPassword' })} />
        <Choice icon={<MonitorSmartphone />} title="Default Encryption" text="Encrypted with your Windows account, no password to type. The file only opens on this PC with this Windows user." onClick={() => run(async () => { await api.chooseProtection(null); onReady() })} />
      </Shell>
    )
  }

  if (step.state === 'newPassword') {
    const mismatch = confirm.length > 0 && password !== confirm
    return (
      <Shell icon={<KeyRound />} title="Choose a password" description="At least 4 characters. It can't be recovered if you forget it.">
        <form className="grid gap-3" onSubmit={(e) => { e.preventDefault(); run(async () => { await api.chooseProtection(password); onReady() }) }}>
          <Input autoFocus type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={t('Password')} />
          <Input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} placeholder={t('Confirm password')} aria-invalid={mismatch} />
          {mismatch && <p className="text-sm text-destructive">{t('The passwords don\'t match')}</p>}
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex gap-2">
            <Button type="button" variant="outline" onClick={() => setStep({ state: 'chooseProtection' })}>{t('Back')}</Button>
            <Button type="submit" className="flex-1" disabled={busy || password.length < 4 || password !== confirm}>
              {busy && <Loader2 className="animate-spin" />} {t('Continue')}
            </Button>
          </div>
        </form>
      </Shell>
    )
  }

  if (step.state === 'needsPassword') {
    return (
      <Shell icon={<LockKeyhole />} title="Restricted Access" description="Enter your password to open your accounts.">
        <form className="grid gap-3" onSubmit={(e) => { e.preventDefault(); run(async () => { await api.unlock(password); onReady() }) }}>
          <Input autoFocus type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder={t('Password')} aria-invalid={!!error} />
          {error && <p className="text-sm text-destructive">{error}</p>}
          <Button type="submit" disabled={busy || password.length === 0}>
            {busy && <Loader2 className="animate-spin" />} {t('Unlock')}
          </Button>
        </form>
      </Shell>
    )
  }

  if (step.state === 'offerBackup') {
    return (
      <Shell icon={<ShieldCheck />} title="No accounts were loaded but there is a backup file" description={step.locked ? 'The existing backup file is password-locked, would you like to attempt to load it?' : 'Would you like to load the backup file?'}>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex gap-2">
          <Button variant="outline" className="flex-1" onClick={onReady}>{t('No')}</Button>
          <Button className="flex-1" disabled={busy} onClick={() => run(async () => { const next = await api.loadBackup(); if (next.state === 'ready') onReady(); else setStep(next) })}>{t('Yes')}</Button>
        </div>
      </Shell>
    )
  }

  if (step.state === 'failed') {
    return (
      <Shell icon={<ShieldCheck />} title="Failed to load accounts!" description="A backup file was created in case the data can be recovered.">
        <p className="rounded-md bg-muted p-3 font-mono text-xs break-words">{step.message}</p>
        <Button onClick={onReady}>{t('Continue')}</Button>
      </Shell>
    )
  }

  return null
}
