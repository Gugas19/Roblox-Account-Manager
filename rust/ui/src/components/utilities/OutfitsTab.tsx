/** Outfits of any player, wear them (or any avatar JSON) on the selected account; missing items can be bought */

import { Loader2, Shirt } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { GameIcon } from '@/components/common/GameTile'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { MissingAsset, Outfit } from '@/lib/types'

export function OutfitsTab() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const { selected } = useStore()
  const [username, setUsername] = useState('')
  const [outfits, setOutfits] = useState<Outfit[]>([])
  const [missing, setMissing] = useState<MissingAsset[]>([])
  const [loading, setLoading] = useState(false)

  const wear = async (json: string) => {
    if (!selected[0]) return notify.info('No Account Selected!')
    try {
      const result = await api.wear(selected[0], json)
      setMissing(result)
      notify.success(result.length ? t('{0} items are missing, you can buy them below', result.length) : 'Avatar changed')
    } catch (e) {
      notify.error(e)
    }
  }

  const buy = async (asset: MissingAsset) => {
    if (!asset.productId || asset.price === null) return
    const ok = await dialogs.confirm({ title: t('Purchase {0}', asset.name), description: t('Are you sure you want to purchase {0} for {1}?', asset.name, `R$ ${asset.price}`) + '\n' + t('This action is irreversable.'), destructive: true })
    if (!ok) return
    try {
      const result = await api.purchase(selected[0], asset.productId, asset.price, asset.sellerId ?? 0)
      if (result.purchased) {
        notify.success(t('Successfully purchased {0} for {1}', asset.name, `R$ ${asset.price}`))
        setMissing((m) => m.filter((a) => a.id !== asset.id))
      } else notify.error(String(result.errorMsg ?? result.title ?? 'Failed'))
    } catch (e) {
      notify.error(e)
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <form className="flex gap-2" onSubmit={async (e) => { e.preventDefault(); setLoading(true); try { setOutfits(await api.outfits(username)) } catch (err) { notify.error(err) } finally { setLoading(false) } }}>
        <Input value={username} onChange={(e) => setUsername(e.target.value)} placeholder={t('Username')} />
        <Button type="submit" disabled={!username.trim()}>{t('View Outfits')}</Button>
        <Button type="button" variant="secondary" disabled={!selected[0]} onClick={async () => { const json = await dialogs.prompt({ title: 'Wear Avatar', label: 'Avatar JSON', multiline: true }); if (json) await wear(json) }}>
          <Shirt /> {t('Wear JSON')}
        </Button>
      </form>

      {missing.length > 0 && (
        <div className="grid gap-2 rounded-lg border border-amber-500/40 bg-amber-500/5 p-3">
          <div className="text-sm font-medium">{t('Missing Items')}</div>
          {missing.map((a) => (
            <div key={a.id} className="flex items-center gap-3">
              <GameIcon src={a.icon} className="size-9" />
              <span className="flex-1 truncate text-sm">{a.name}</span>
              <Button size="sm" disabled={!a.forSale || a.price === null} onClick={() => buy(a)}>{a.forSale && a.price !== null ? `R$ ${a.price}` : t('Not for sale')}</Button>
            </div>
          ))}
        </div>
      )}

      <div className="min-h-0 flex-1">
        {loading ? (
          <div className="grid place-items-center py-16"><Loader2 className="animate-spin text-muted-foreground" /></div>
        ) : (
          <ScrollArea className="h-full">
            <div className="grid grid-cols-4 gap-2 pr-3">
              {outfits.map((o) => (
                <button key={o.id} className="grid gap-1 rounded-lg border p-2 text-left text-xs transition-colors hover:border-primary disabled:opacity-50" disabled={!selected[0]} title={t('Wear on the selected account')} onClick={async () => wear(await api.outfitJson(o.id))}>
                  <GameIcon src={o.icon} className="aspect-square w-full" />
                  <span className="truncate">{o.name}</span>
                </button>
              ))}
            </div>
            {outfits.length === 0 && <p className="py-16 text-center text-sm text-muted-foreground">{t('Type a username to see their outfits, click one to wear it on the selected account.')}</p>}
          </ScrollArea>
        )}
      </div>
    </div>
  )
}
