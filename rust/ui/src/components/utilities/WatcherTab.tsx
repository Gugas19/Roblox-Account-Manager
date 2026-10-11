/** Roblox Watcher: closes Roblox windows that are stuck, disconnected or using too little memory */

import { FolderOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { NumberSetting, SettingRow, TextSetting, Toggle } from '@/components/common/SettingRow'
import { Switch } from '@/components/ui/switch'
import { ScrollArea } from '@/components/ui/scroll-area'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { useStore } from '@/lib/store'

export function WatcherTab() {
  const t = useT()
  const { flag, updateSetting } = useStore()
  const enabled = flag('Watcher', 'Enabled')

  return (
    <ScrollArea className="h-full pr-3">
      <div className="divide-y">
        <SettingRow title="Enable Roblox Watcher" hint="Watches the Roblox windows opened by the app. The window you're using is never touched.">
          <Switch checked={enabled} onCheckedChange={(on) => { void updateSetting('Watcher', 'Enabled', on ? 'true' : 'false'); void api.watcherEnable(on) }} />
        </SettingRow>
        <div className={enabled ? '' : 'pointer-events-none opacity-50'}>
          <Toggle section="Watcher" name="ExitOnBeta" title="Exit if Beta Home Menu Detected" hint="Closes Roblox when it goes back to the home menu after a game." />
          <Toggle section="Watcher" name="VerifyDataModel" title="Verify exit signal" hint="Only when the game really ended, fewer false alarms." />
          <Toggle section="Watcher" name="ExitIfNoConnection" title="Exit if No Connection to Server" />
          <NumberSetting section="Watcher" name="NoConnectionTimeout" title="Seconds without connection" min={5} max={3600} fallback={60} />
          <Toggle section="Watcher" name="CloseRbxMemory" title="Close Roblox if memory is below" hint="A window using very little memory is usually stuck or crashed." />
          <NumberSetting section="Watcher" name="MemoryLowValue" title="Memory (MB)" min={50} max={10000} fallback={200} />
          <Toggle section="Watcher" name="CloseRbxWindowTitle" title="Close Roblox if the window title isn't" />
          <TextSetting section="Watcher" name="ExpectedWindowTitle" title="Expected window title" placeholder="Roblox" width="w-40" />
          <Toggle section="Watcher" name="SaveWindowPositions" title="Save Window Positions" hint="Each account's Roblox window opens where it was last time." />
          <Toggle section="Watcher" name="IgnoreExistingProcesses" title="Ignore already running processes" hint="Roblox windows open before the watcher started are left alone." />
          <NumberSetting section="Watcher" name="ScanInterval" title="Scan Interval (s)" min={1} max={120} fallback={6} />
          <NumberSetting section="Watcher" name="ReadInterval" title="Log Read Interval (ms)" min={50} max={10000} fallback={250} />
        </div>
        <SettingRow title="Roblox logs" hint="What the watcher reads.">
          <Button variant="secondary" onClick={() => api.open('logs')}><FolderOpen /> {t('Open')}</Button>
        </SettingRow>
      </div>
    </ScrollArea>
  )
}
