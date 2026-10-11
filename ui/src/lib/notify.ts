/** Toasts. Messages are English keys (from the interface or from Rust) and get translated */

import { toast } from 'sonner'
import type { Translate } from './i18n'

export function notifier(t: Translate) {
  return {
    success: (message: string) => toast.success(t(message)),
    info: (message: string) => toast(t(message)),
    error: (error: unknown) => toast.error(t(error instanceof Error ? error.message : String(error)), { duration: 8000 }),
  }
}
