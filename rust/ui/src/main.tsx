import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from '@/App'
import { api } from '@/lib/api'
import { applyTheme } from '@/lib/theme'
import './index.css'

// the window shows once the first screen is ready, no white flash
api.bootstrap().then((boot) => {
  applyTheme(boot.theme)
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App boot={boot} />
    </StrictMode>,
  )
})
