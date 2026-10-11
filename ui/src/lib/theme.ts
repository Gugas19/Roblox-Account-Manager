/** Applies the RAMTheme.ini colors (same keys as the C# theme editor) to the shadcn/ui CSS variables */

import type { Theme } from './types'

function hex(value: string | undefined, fallback: string) {
  return value && /^#[0-9a-f]{6}$/i.test(value) ? value : fallback
}

/** Lighter or darker version of a color, for hover states and borders */
function shift(color: string, amount: number) {
  const n = parseInt(color.slice(1), 16)
  const channel = (c: number) => Math.round(Math.min(255, Math.max(0, c + (amount > 0 ? (255 - c) * amount : c * amount))))
  const r = channel((n >> 16) & 255)
  const g = channel((n >> 8) & 255)
  const b = channel(n & 255)
  return `#${((1 << 24) | (r << 16) | (g << 8) | b).toString(16).slice(1)}`
}

function brightness(color: string) {
  const n = parseInt(color.slice(1), 16)
  return (0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)) / 255
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement
  const background = hex(theme.FormsBG, '#14161c')
  const foreground = hex(theme.FormsFG, '#e6e8ee')
  const card = hex(theme.AccountsBG, '#1a1c24')
  const input = hex(theme.TextBoxesBG, '#1f2230')
  const button = hex(theme.ButtonsBG, '#262935')
  const accent = hex(theme.AccentColor, '#4f7cff')
  const dark = brightness(background) < 0.5

  root.classList.toggle('dark', dark)

  const vars: Record<string, string> = {
    '--background': background,
    '--foreground': foreground,
    '--card': card,
    '--card-foreground': hex(theme.AccountsFG, foreground),
    '--popover': shift(card, dark ? 0.04 : -0.02),
    '--popover-foreground': foreground,
    '--primary': accent,
    '--primary-foreground': brightness(accent) > 0.62 ? '#14161c' : '#ffffff',
    '--secondary': button,
    '--secondary-foreground': hex(theme.ButtonsFG, foreground),
    '--muted': shift(card, dark ? 0.06 : -0.04),
    '--muted-foreground': shift(foreground, dark ? -0.42 : 0.42),
    '--accent': shift(card, dark ? 0.08 : -0.05),
    '--accent-foreground': foreground,
    '--border': shift(background, dark ? 0.1 : -0.1),
    '--input': shift(input, dark ? 0.1 : -0.1),
    '--input-background': input,
    '--ring': accent,
  }

  for (const [key, value] of Object.entries(vars)) root.style.setProperty(key, value)
}
