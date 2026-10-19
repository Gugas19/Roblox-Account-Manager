/** Avatar page: outfits of any player (worn by the selected account), and what the accounts own */

import { Page } from '@/components/layout/Page'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { InventoryTab } from '@/components/utilities/InventoryTab'
import { OutfitsTab } from '@/components/utilities/OutfitsTab'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function AvatarPage() {
  const t = useT()
  const { pageTab } = useStore()
  return (
    <Page name="avatar" title="Avatar" description="Outfits of any player, worn by the selected account, and what your accounts own.">
      <Tabs key={pageTab} defaultValue={pageTab ?? 'outfits'} className="flex min-h-0 flex-1 flex-col">
        <TabsList>
          <TabsTrigger value="outfits">{t('Outfits')}</TabsTrigger>
          <TabsTrigger value="inventory">{t('Inventory')}</TabsTrigger>
        </TabsList>
        <TabsContent value="outfits" className="mt-3 min-h-0 flex-1">
          <OutfitsTab />
        </TabsContent>
        <TabsContent value="inventory" className="mt-3 min-h-0 flex-1">
          <InventoryTab />
        </TabsContent>
      </Tabs>
    </Page>
  )
}
