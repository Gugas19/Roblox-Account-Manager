/** The navigation rail on the left: one icon per page, Settings at the bottom */

import { Building2, Gamepad2, Handshake, History, MonitorPlay, Settings, Shirt, ShieldCheck, Users, type LucideIcon } from 'lucide-react'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useT } from '@/lib/i18n'
import { useStore, type Page } from '@/lib/store'

const PAGES: [Page, string, LucideIcon][] = [
  ['accounts', 'Accounts', Users],
  ['instances', 'Instances', MonitorPlay],
  ['friends', 'Friends', Handshake],
  ['groups', 'Groups', Building2],
  ['games', 'Games', Gamepad2],
  ['avatar', 'Avatar', Shirt],
  ['antiafk', 'Anti-AFK', ShieldCheck],
  ['history', 'History', History],
]

function RailButton({ page, label, icon: Icon }: { page: Page; label: string; icon: LucideIcon }) {
  const t = useT()
  const { page: current, setPage } = useStore()
  const active = current === page
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          aria-label={t(label)}
          aria-current={active ? 'page' : undefined}
          data-nav={page}
          onClick={() => setPage(page)}
          className={`grid size-10 cursor-pointer place-items-center rounded-lg transition-colors ${active ? 'bg-primary/15 text-primary' : 'text-muted-foreground hover:bg-accent hover:text-foreground'}`}
        >
          <Icon className="size-5" />
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{t(label)}</TooltipContent>
    </Tooltip>
  )
}

export function NavRail() {
  return (
    <nav className="flex w-14 shrink-0 flex-col items-center gap-1 pb-4" aria-label="Pages">
      {PAGES.map(([page, label, icon]) => (
        <RailButton key={page} page={page} label={label} icon={icon} />
      ))}
      <div className="mt-auto">
        <RailButton page="settings" label="Settings" icon={Settings} />
      </div>
    </nav>
  )
}
