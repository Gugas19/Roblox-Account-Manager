/**
 * Translations: the same JSON tables as the C# version, the English text is the key.
 * Text built from a template ("Added {0}") is translated too, also when it comes back from Rust already filled in.
 */

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react'

type Table = Record<string, string>

interface Template {
  pattern: RegExp
  translation: string
  slots: number[]
}

function escape(text: string) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function buildTemplates(table: Table): Template[] {
  return Object.entries(table)
    .filter(([key]) => /\{\d+\}/.test(key))
    .map(([key, translation]) => {
      const slots: number[] = []
      const source = escape(key).replace(/\\\{(\d+)\\\}/g, (_, n) => {
        slots.push(Number(n))
        return '([\\s\\S]*?)'
      })
      return { pattern: new RegExp(`^${source}$`), translation, slots }
    })
    .sort((a, b) => b.pattern.source.length - a.pattern.source.length)
}

export function createTranslator(table: Table) {
  const templates = buildTemplates(table)

  const translate = (text: string, depth = 0): string => {
    if (!text) return text
    const direct = table[text]
    if (direct) return direct

    const trimmed = text.trim()
    if (trimmed !== text && table[trimmed]) return text.replace(trimmed, table[trimmed])

    if (depth < 2) {
      for (const t of templates) {
        const m = t.pattern.exec(text)
        if (!m) continue
        return t.translation.replace(/\{(\d+)\}/g, (_, n) => {
          const index = t.slots.indexOf(Number(n))
          return index >= 0 ? translate(m[index + 1], depth + 1) : ''
        })
      }
    }
    return text
  }

  /** t('Added {0}', name) fills the template after translating it */
  return (text: string, ...args: (string | number)[]) => {
    if (args.length === 0) return translate(text)
    const translated = table[text] ?? text
    return translated.replace(/\{(\d+)\}/g, (_, n) => String(args[Number(n)] ?? ''))
  }
}

export type Translate = ReturnType<typeof createTranslator>

interface I18n {
  t: Translate
  language: string
  setTable: (language: string, table: string) => void
}

const Context = createContext<I18n>({ t: (text) => text, language: 'en', setTable: () => {} })

export function I18nProvider({ language: initialLanguage, table: initialTable, children }: { language: string; table: string; children: ReactNode }) {
  const [state, setState] = useState(() => ({ language: initialLanguage, table: JSON.parse(initialTable || '{}') as Table }))

  const setTable = useCallback((language: string, table: string) => setState({ language, table: JSON.parse(table || '{}') }), [])
  const t = useMemo(() => createTranslator(state.table), [state.table])

  return <Context.Provider value={{ t, language: state.language, setTable }}>{children}</Context.Provider>
}

export const useI18n = () => useContext(Context)
export const useT = () => useContext(Context).t
