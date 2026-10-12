/** Account Utilities for the selected account: password, email, display name, friends/blocks, PIN, privacy, sessions */

import { Ban, KeyRound, LockOpen, LogOut, Mail, PenLine, UserPlus } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

const EMPTY = { current: '', next: '', email: '', display: '', target: '', pin: '' }

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  const t = useT()
  return (
    <section className="grid gap-2">
      <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{t(title)}</div>
      <div className="flex gap-2">{children}</div>
    </section>
  )
}

export function AccountUtilsDialog() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const { panel, setPanel, selectedAccounts } = useStore()
  const account = selectedAccounts[0]
  const [form, setForm] = useState(EMPTY)
  const [busy, setBusy] = useState(false)
  const set = (key: keyof typeof form) => (e: React.ChangeEvent<HTMLInputElement>) => setForm({ ...form, [key]: e.target.value })

  // nothing typed (passwords!) stays behind once the window closes or another account is picked
  useEffect(() => setForm(EMPTY), [panel, account?.username])

  /** One request at a time, so nothing is sent twice */
  const run = async (work: () => Promise<unknown>, success?: string) => {
    setBusy(true)
    try {
      const result = await work()
      notify.success(typeof result === 'string' ? result : success ?? 'Done')
    } catch (e) {
      notify.error(e)
    } finally {
      setBusy(false)
    }
  }

  if (!account) return null
  const name = account.username

  return (
    <Dialog open={panel === 'accountUtils'} onOpenChange={(open) => !open && setPanel(null)}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t('Account Utilities')}</DialogTitle>
          <DialogDescription>{t('Changes apply to {0}', name)}</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4">
          <Section title="Password">
            <Input type="password" value={form.current} onChange={set('current')} placeholder={t('Current Password')} />
            <Input type="password" value={form.next} onChange={set('next')} placeholder={t('New Password')} />
            <Button variant="secondary" disabled={busy || !form.current || !form.next} onClick={() => run(() => api.changePassword(name, form.current, form.next))}>
              <KeyRound /> {t('Change Password')}
            </Button>
          </Section>

          <Section title="Email">
            <Input type="email" value={form.email} onChange={set('email')} placeholder={t('New email')} />
            <Button variant="secondary" disabled={busy || !form.current || !form.email} title={t('Uses the current password above')} onClick={() => run(() => api.changeEmail(name, form.current, form.email), 'Email changed!')}>
              <Mail /> {t('Change Email')}
            </Button>
          </Section>

          <Section title="DISPLAY NAME">
            <Input value={form.display} onChange={set('display')} placeholder={t('Display name')} />
            <Button variant="secondary" disabled={busy || !form.display} onClick={() => run(() => api.setDisplayName(name, form.display), t('Successfully set {0}\'s Display Name to {1}', name, form.display))}>
              <PenLine /> {t('Set Display Name')}
            </Button>
          </Section>

          <Section title="OTHER PLAYERS">
            <Input value={form.target} onChange={set('target')} placeholder={t('Target Username')} />
            <Button variant="secondary" disabled={busy || !form.target} onClick={() => run(() => api.addFriend(name, form.target), t('Friend request sent to {0}', form.target))}>
              <UserPlus /> {t('Add Friend')}
            </Button>
            <Button variant="secondary" disabled={busy || !form.target} onClick={() => run(async () => ((await api.toggleBlock(name, form.target)) ? t('Unblocked {0}', form.target) : t('Blocked {0}', form.target)))}>
              <Ban /> {t('Block')}
            </Button>
          </Section>

          <Separator />

          <div className="grid grid-cols-2 gap-4">
            <Section title="Security">
              <Input className="w-24" inputMode="numeric" maxLength={4} value={form.pin} onChange={set('pin')} placeholder={t('PIN')} />
              <Button variant="secondary" disabled={busy || form.pin.length !== 4} onClick={() => run(() => api.unlockPin(name, form.pin), 'Pin unlocked for 5 minutes')}>
                <LockOpen /> {t('Unlock')}
              </Button>
            </Section>

            <Section title="Who Can Follow">
              <Select disabled={busy} onValueChange={(v) => run(() => api.setFollowPrivacy(name, Number(v)), 'Saved')}>
                <SelectTrigger className="w-full"><SelectValue placeholder={t('Choose')} /></SelectTrigger>
                <SelectContent>
                  {['Everyone', 'Friends, Followed, Followers', 'Friends, Followed', 'Friends', 'No one'].map((label, i) => (
                    <SelectItem key={label} value={String(i)}>{t(label)}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Section>
          </div>

          <Button
            variant="outline"
            className="justify-self-start"
            disabled={busy}
            onClick={async () => {
              if (await dialogs.confirm({ title: 'Are you sure you want sign out of all other sessions?', destructive: true })) await run(() => api.signOutOthers(name))
            }}
          >
            <LogOut /> {t('Sign out of all other sessions')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
