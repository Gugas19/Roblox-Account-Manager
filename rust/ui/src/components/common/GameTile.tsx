/** A game with its icon, used by recent games, search, favorites and universes */

import { Gamepad2, Users } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Game } from '@/lib/types'

export function GameIcon({ src, className = 'size-10' }: { src?: string | null; className?: string }) {
  return src ? (
    <img src={src} alt="" loading="lazy" className={`${className} shrink-0 rounded-md bg-muted object-cover`} />
  ) : (
    <div className={`${className} grid shrink-0 place-items-center rounded-md bg-muted text-muted-foreground`}>
      <Gamepad2 className="size-1/2" />
    </div>
  )
}

export function GameTile({ game, onClick, children }: { game: Game; onClick?: () => void; children?: ReactNode }) {
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => e.key === 'Enter' && onClick?.()}
      className="group flex items-center gap-3 rounded-lg border bg-card p-2 text-left transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
    >
      <GameIcon src={game.icon} className="size-12" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{game.name}</div>
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span>{game.placeId}</span>
          {game.players !== undefined && (
            <span className="flex items-center gap-1">
              <Users className="size-3" /> {game.players.toLocaleString()}
            </span>
          )}
          {!!game.likes && <span>👍 {game.likes}%</span>}
        </div>
      </div>
      {children}
    </div>
  )
}
