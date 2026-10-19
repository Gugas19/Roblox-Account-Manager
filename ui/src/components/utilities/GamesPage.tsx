/** Games page: servers of the chosen game, games to find, favorites and universes */

import { Page } from '@/components/layout/Page'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { FavoritesTab, GamesTab, UniverseTab } from '@/components/utilities/GamesTabs'
import { ServersTab } from '@/components/utilities/ServersTab'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function GamesPage() {
  const t = useT()
  const { pageTab } = useStore()

  return (
    <Page name="games" title="Games" description="Servers of the chosen game, games to find, your favorites and universes.">
      <Tabs key={pageTab} defaultValue={pageTab ?? 'servers'} className="flex min-h-0 flex-1 flex-col">
        <TabsList>
          <TabsTrigger value="servers">{t('Servers')}</TabsTrigger>
          <TabsTrigger value="games">{t('Games')}</TabsTrigger>
          <TabsTrigger value="favorites">{t('Favorites')}</TabsTrigger>
          <TabsTrigger value="universe">{t('Universe')}</TabsTrigger>
        </TabsList>
        {[
          ['servers', <ServersTab key="s" />],
          ['games', <GamesTab key="g" />],
          ['favorites', <FavoritesTab key="f" />],
          ['universe', <UniverseTab key="u" />],
        ].map(([value, content]) => (
          <TabsContent key={value as string} value={value as string} className="mt-3 min-h-0 flex-1">
            {content}
          </TabsContent>
        ))}
      </Tabs>
    </Page>
  )
}
