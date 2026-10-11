/** Right side: the game, following a player, the selected account, and the two utility windows */

import { Save, Tag, UserRoundSearch, Wrench, UserCog } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Separator } from '@/components/ui/separator'
import { Textarea } from '@/components/ui/textarea'
import { GamePanel } from '@/components/sidebar/GamePanel'
import { useActions } from '@/hooks/useActions'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

function Heading({ children }: { children: React.ReactNode }) {
  return <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{children}</div>
}

export function Sidebar() {
  const t = useT()
  const actions = useActions()
  const { selectedAccounts, boot, setPanel } = useStore()
  const [follow, setFollow] = useState(boot.followUser)
  const [alias, setAlias] = useState('')
  const [description, setDescription] = useState('')
  const one = selectedAccounts.length === 1 ? selectedAccounts[0] : null
  const none = selectedAccounts.length === 0

  useEffect(() => {
    setAlias(one?.alias ?? '')
    setDescription(one?.description ?? '')
  }, [one?.username, one?.alias, one?.description])

  return (
    <aside className="flex h-full min-h-0 flex-col gap-4 overflow-y-auto p-4">
      <GamePanel />

      <Separator />

      <section className="grid gap-2">
        <Heading>{t('FOLLOW PLAYER')}</Heading>
        <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); actions.follow(follow) }}>
          <Input value={follow} onChange={(e) => setFollow(e.target.value)} placeholder={t('Username')} />
          <Button type="submit" variant="secondary" disabled={none || !follow.trim()}>
            <UserRoundSearch /> {t('Follow')}
          </Button>
        </form>
      </section>

      <Separator />

      <section className="grid min-h-0 flex-1 grid-rows-[auto_auto_1fr_auto] gap-2">
        <Heading>{none ? t('SELECTED ACCOUNT') : one ? one.username : t('{0} selected', selectedAccounts.length)}</Heading>
        <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); actions.setAlias(alias) }}>
          <Input value={alias} maxLength={50} disabled={none} onChange={(e) => setAlias(e.target.value)} placeholder={t('Alias')} />
          <Button type="submit" variant="secondary" disabled={none}>
            <Tag /> {t('Set Alias')}
          </Button>
        </form>
        <Textarea className="min-h-24 resize-none" value={description} maxLength={5000} disabled={none} onChange={(e) => setDescription(e.target.value)} placeholder={t('Description')} />
        <Button variant="secondary" disabled={none} onClick={() => actions.setDescription(description)}>
          <Save /> {t('Set Description')}
        </Button>
      </section>

      <div className="grid grid-cols-2 gap-2">
        <Button variant="outline" onClick={() => setPanel('utilities')}>
          <Wrench /> {t('Utilities')}
        </Button>
        <Button variant="outline" disabled={selectedAccounts.length !== 1} onClick={() => setPanel('accountUtils')}>
          <UserCog /> {t('Account Utilities')}
        </Button>
      </div>
    </aside>
  )
}
