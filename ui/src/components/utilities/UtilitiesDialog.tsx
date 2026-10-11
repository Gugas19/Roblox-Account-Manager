/** Utilities window: servers, games, favorites, universes, outfits and the watcher */

import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { FavoritesTab, GamesTab, UniverseTab } from '@/components/utilities/GamesTabs'
import { OutfitsTab } from '@/components/utilities/OutfitsTab'
import { ServersTab } from '@/components/utilities/ServersTab'
import { WatcherTab } from '@/components/utilities/WatcherTab'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function UtilitiesDialog() {
  const t = useT()
  const { panel, setPanel } = useStore()

  return (
    <Dialog open={panel === 'utilities'} onOpenChange={(open) => !open && setPanel(null)}>
      <DialogContent className="flex h-[82vh] flex-col sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{t('Utilities')}</DialogTitle>
          <DialogDescription className="sr-only">{t('Utilities')}</DialogDescription>
        </DialogHeader>
        <Tabs defaultValue="servers" className="flex min-h-0 flex-1 flex-col">
          <TabsList>
            <TabsTrigger value="servers">{t('Servers')}</TabsTrigger>
            <TabsTrigger value="games">{t('Games')}</TabsTrigger>
            <TabsTrigger value="favorites">{t('Favorites')}</TabsTrigger>
            <TabsTrigger value="universe">{t('Universe')}</TabsTrigger>
            <TabsTrigger value="outfits">{t('Outfits')}</TabsTrigger>
            <TabsTrigger value="watcher">{t('Watcher')}</TabsTrigger>
          </TabsList>
          {[
            ['servers', <ServersTab key="s" />],
            ['games', <GamesTab key="g" />],
            ['favorites', <FavoritesTab key="f" />],
            ['universe', <UniverseTab key="u" />],
            ['outfits', <OutfitsTab key="o" />],
            ['watcher', <WatcherTab key="w" />],
          ].map(([value, content]) => (
            <TabsContent key={value as string} value={value as string} className="mt-3 min-h-0 flex-1">
              {content}
            </TabsContent>
          ))}
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
