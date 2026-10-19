/** Avatar page: outfits of any player, worn by the selected account */

import { Page } from '@/components/layout/Page'
import { OutfitsTab } from '@/components/utilities/OutfitsTab'

export function AvatarPage() {
  return (
    <Page name="avatar" title="Avatar" description="Outfits of any player, worn by the selected account. Missing items can be bought.">
      <div className="min-h-0 flex-1">
        <OutfitsTab />
      </div>
    </Page>
  )
}
