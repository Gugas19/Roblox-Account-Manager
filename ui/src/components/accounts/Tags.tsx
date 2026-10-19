/** Account tags: a color of their own from the tag's name, the same everywhere */

const COLORS = ['#3b82f6', '#22c55e', '#f59e0b', '#ec4899', '#8b5cf6', '#14b8a6', '#ef4444', '#64748b']

export function tagColor(tag: string) {
  let hash = 0
  for (const c of tag) hash = (hash * 31 + c.charCodeAt(0)) | 0
  return COLORS[Math.abs(hash) % COLORS.length]
}

/** Up to `max` tags, then +N */
export function TagChips({ tags, max = 2 }: { tags: string[]; max?: number }) {
  if (!tags.length) return null
  return (
    <span className="flex min-w-0 shrink-[4] items-center gap-1 overflow-hidden">
      {tags.slice(0, max).map((tag) => (
        <span key={tag} className="truncate rounded px-1.5 text-[10px] leading-4 font-medium text-white" style={{ background: tagColor(tag) }} title={tag}>
          {tag}
        </span>
      ))}
      {tags.length > max && <span className="text-[10px] text-muted-foreground" title={tags.slice(max).join(', ')}>+{tags.length - max}</span>}
    </span>
  )
}
