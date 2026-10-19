/** Settings: General, Developer and Miscellaneous, every change applies right away */

import { AlertTriangle, Download, FileJson, FolderOpen, Lock, Upload, X } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Flag } from '@/components/common/Flag'
import { NumberSetting, SettingRow, TextSetting, Toggle } from '@/components/common/SettingRow'
import { UpdatesSection } from './UpdatesSection'
import { ThemeSection } from './ThemeSection'
import { Page } from '@/components/layout/Page'
import { WatcherTab } from '@/components/utilities/WatcherTab'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { api } from '@/lib/api'
import { useI18n } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

/** Changing between Windows and password protection (asks the current password first) */
function useProtectionChange() {
  const { t } = useI18n()
  const dialogs = useDialogs()
  const notify = notifier(t)

  return async () => {
    const current = await api.protection()
    if (current.hasPassword) {
      const password = await dialogs.prompt({ title: 'Change Encryption Method', label: 'Enter your current password', password: true })
      if (password === null) return
      try {
        await api.checkPassword(password)
      } catch (e) {
        return notify.error(e)
      }
    }

    const usePassword = await dialogs.confirm({
      title: 'Change Encryption Method',
      description: 'Password protected: works on any PC, asks for the password when the app starts.\nDefault encryption: no password, only opens on this PC with this Windows user.',
      confirm: 'Password protected',
      cancel: 'Default Encryption',
    })

    if (!usePassword) {
      await api.chooseProtection(null)
      return notify.success('Saved')
    }

    const first = await dialogs.prompt({ title: 'Choose a password', label: 'At least 4 characters. It can\'t be recovered if you forget it.', password: true })
    if (!first) return
    const second = await dialogs.prompt({ title: 'Please confirm your password.', password: true })
    if (second === null) return
    if (first !== second) return notify.error('You have entered the wrong password, please try again.')
    try {
      await api.chooseProtection(first)
      notify.success('Saved')
    } catch (e) {
      notify.error(e)
    }
  }
}

/** Why Multi Roblox isn't working yet, checked while the settings are open */
function MultiRobloxStatus() {
  const t = useI18n().t
  const notify = notifier(t)
  const [status, setStatus] = useState<{ enabled: boolean; working: boolean; tray: boolean } | null>(null)
  useEffect(() => {
    const look = () => api.multiRobloxStatus().then(setStatus, () => {})
    look()
    const timer = setInterval(look, 2000)
    return () => clearInterval(timer)
  }, [])
  if (!status?.enabled || status.working) return null
  const closeTray = async () => {
    try {
      if (await api.closeRobloxTray()) notify.success('Multi Roblox works now')
      else notify.error('Roblox still holds it: close every Roblox window')
    } catch (e) {
      notify.error(e)
    }
    setStatus(await api.multiRobloxStatus())
  }
  return (
    <div className="mb-2 flex items-center gap-3 rounded-lg border border-amber-500/40 bg-amber-500/5 p-3 text-sm" data-testid="multi-roblox-status">
      <AlertTriangle className="size-4 shrink-0 text-amber-500" />
      <span className="flex-1">
        {status.tray
          ? t('Roblox is open in the background (it starts with Windows) and stops Multi Roblox.')
          : t('Multi Roblox starts working once every Roblox window is closed.')}
      </span>
      {status.tray && (
        <Button size="sm" variant="secondary" onClick={closeTray}>
          <X /> {t('Close it')}
        </Button>
      )}
    </div>
  )
}

export function SettingsPage() {
  const { t, language, setTable } = useI18n()
  const { pageTab, boot, setting, updateSetting, flag, accounts } = useStore()
  const [startup, setStartup] = useState(boot.startup)
  const dialogs = useDialogs()
  const notify = notifier(t)

  // turning "Save Passwords" off offers to forget the ones already saved
  const savePasswordsChanged = async (on: boolean) => {
    if (on || !accounts.some((a) => a.hasPassword)) return
    const remove = await dialogs.confirm({ title: 'Remove saved passwords?', description: 'Do you also want to remove every password that is already saved?', destructive: true })
    if (remove) notify.success(t('Removed the password of {0} accounts', await api.clearSavedPasswords()))
  }

  // backups: always encrypted, with a password chosen here
  const exportBackup = async () => {
    const password = await dialogs.prompt({ title: 'Export accounts', label: "Password for the backup, at least 4 characters. It can't be recovered if you forget it.", password: true })
    if (!password) return
    const again = await dialogs.prompt({ title: 'Export accounts', label: 'Confirm password', password: true })
    if (again === null) return
    if (again !== password) return notify.error("The passwords don't match")
    try {
      const path = await api.exportAccounts(password)
      if (path) notify.success(t('Saved to {0}', path))
    } catch (e) {
      notify.error(e)
    }
  }
  const importBackup = async () => {
    const path = await api.pickFile(t('Import accounts'))
    if (!path) return
    const password = await dialogs.prompt({ title: 'Import accounts', label: "The backup's password", password: true })
    if (!password) return
    try {
      const r = await api.importBackup(path, password)
      notify.success(t('{0} accounts added, {1} were already here', r.added, r.existing))
    } catch (e) {
      notify.error(e)
    }
  }
  const changeProtection = useProtectionChange()
  const customSettings = setting('General', 'CustomClientSettings')

  const changeLanguage = async (code: string) => {
    const result = await api.setLanguage(code)
    setTable(result.language, result.table)
  }

  return (
    <Page name="settings" title="Settings" description="Changes are saved and applied right away.">
        <Tabs key={pageTab} defaultValue={pageTab ?? 'general'} className="flex min-h-0 flex-1 flex-col">
          <TabsList>
            <TabsTrigger value="general">{t('General')}</TabsTrigger>
            <TabsTrigger value="appearance">{t('Appearance')}</TabsTrigger>
            <TabsTrigger value="watcher">{t('Watcher')}</TabsTrigger>
            <TabsTrigger value="developer">{t('Developer')}</TabsTrigger>
            <TabsTrigger value="misc">{t('Miscellaneous')}</TabsTrigger>
          </TabsList>

          <ScrollArea className="mt-2 min-h-0 flex-1 pr-3">
            {/* room at the end: the last rows can scroll up above the notifications in the corner */}
            <div className="mx-auto max-w-3xl pb-24">
            <TabsContent value="appearance">
              <ThemeSection />
            </TabsContent>
            <TabsContent value="watcher">
              <WatcherTab />
            </TabsContent>
            <TabsContent value="general" className="divide-y">
              <SettingRow title="Language">
                <Select value={language} onValueChange={changeLanguage}>
                  <SelectTrigger className="w-56">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {boot.languages.map((l) => (
                      <SelectItem key={l.code} value={l.code}>
                        <Flag code={l.flag} /> {l.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </SettingRow>
              <Toggle section="General" name="EnableMultiRbx" title="Multi Roblox" hint="Play several accounts at the same time. Works once every Roblox window is closed." />
              <MultiRobloxStatus />
              <Toggle section="General" name="RenameRobloxWindows" title="Name Roblox windows after their account" hint="Each Roblox window the app opens shows the account (and its alias) in its title, to tell them apart." />
              <Toggle section="General" name="HideFromCaptures" title="Hide this app from screen captures" hint="Screenshots, recordings and screen sharing (OBS, Discord, the Snipping Tool) leave this window out. Roblox itself still shows." />
              <Toggle section="General" name="AsyncJoin" title="Async Launching" hint="When launching several accounts, start the next one as soon as the previous opened instead of waiting the delay." />
              <NumberSetting section="General" name="AccountJoinDelay" title="Launch Delay" hint="Seconds between accounts when launching several." min={0} max={120} fallback={8} />
              <Toggle section="General" name="SavePasswords" title="Save Passwords" hint="Remember the password typed when logging in, to copy it later or log in again when the cookie expires. Off by default." onChange={savePasswordsChanged} />
              <Toggle section="General" name="AutoCookieRefresh" title="Auto Refresh Cookies" hint="Accounts unused for 20 days get a new cookie so they don't get logged out (signs out their other sessions)." />
              <Toggle section="General" name="DisableAgingAlert" title="Disable Aging Alert" hint="Hide the yellow/red dot on accounts unused for over 20 days." />
              <Toggle section="General" name="ShuffleChoosesLowestServer" title="Shuffle Chooses Lowest Server" hint="Shuffle picks the emptiest server instead of a random one." />
              <Toggle section="General" name="AutoCloseLastProcess" title="Close the account's previous Roblox" hint="Avoids the 'same account launched' error when joining again." />
              <SettingRow title="Run on Windows Startup">
                <Switch checked={startup} onCheckedChange={(on) => api.setStartup(on).then(() => setStartup(on))} />
              </SettingRow>
              <TextSetting section="General" name="ServerRegionFormat" title="Region Format" hint="How server regions show, see ip-api.com for the available <fields>." placeholder="<city>, <countryCode>" />
              <NumberSetting section="General" name="MaxRecentGames" title="Max Recent Games" min={1} max={50} fallback={8} />
              <SettingRow title="Account protection" hint="Windows encryption or a password.">
                <Button variant="secondary" onClick={changeProtection}><Lock /> {t('Change Encryption Method')}</Button>
              </SettingRow>
              <SettingRow title="Backup" hint="A copy of your accounts encrypted with a password you choose, to keep safe or to move to another PC.">
                <div className="flex gap-2">
                  <Button variant="secondary" onClick={exportBackup}><Download /> {t('Export')}</Button>
                  <Button variant="secondary" onClick={importBackup}><Upload /> {t('Import')}</Button>
                </div>
              </SettingRow>
              <UpdatesSection />
            </TabsContent>

            <TabsContent value="developer" className="divide-y">
              <Toggle section="Developer" name="DevMode" title="Enable Developer Mode" hint="Extra account menu items: fields, authentication tickets, cookies and launch links." />
              <Toggle section="Developer" name="EnableWebServer" title="Enable Web Server" hint="A local API other programs and scripts can use." />
              <NumberSetting section="WebServer" name="WebServerPort" title="Port" min={1} max={65535} fallback={7963} />
              <TextSetting section="WebServer" name="Password" title="Webserver Password" hint="6 or more characters, required by the protected methods." password />
              <Toggle section="WebServer" name="EveryRequestRequiresPassword" title="Every Request Requires Password" />
              <Toggle section="WebServer" name="AllowGetCookie" title="Allow GetCookie Method" />
              <Toggle section="WebServer" name="AllowGetAccounts" title="Allow GetAccounts Method" />
              <Toggle section="WebServer" name="AllowLaunchAccount" title="Allow LaunchAccount Method" />
              <Toggle section="WebServer" name="AllowAccountEditing" title="Allow Account Modification Methods" />
              <Toggle section="WebServer" name="AllowExternalConnections" title="Allow External Connections" hint="Accept requests from other devices on your network, not only this PC." />
              {boot.webServer && <p className="py-2 text-xs text-muted-foreground">{t('Web server: http://{0}/', boot.webServer)}</p>}
            </TabsContent>

            <TabsContent value="misc" className="divide-y">
              <Toggle section="General" name="ShowPresence" title="Show Account Presence" hint="Colored dot when an account is online, in game or in Studio." />
              <NumberSetting section="General" name="PresenceUpdateRate" title="Refresh (min)" min={1} max={60} fallback={5} />
              <SettingRow title="Use Custom ClientAppSettings" hint={customSettings || 'Copy your own ClientAppSettings.json into Roblox before every launch. Roblox only applies the flags on its allowlist.'}>
                <div className="flex gap-2">
                  {customSettings && <Button variant="ghost" onClick={() => updateSetting('General', 'CustomClientSettings', '')}>{t('Remove')}</Button>}
                  <Button variant="secondary" onClick={async () => { const file = await api.pickFile(t('Use Custom ClientAppSettings')); if (file) void updateSetting('General', 'CustomClientSettings', file) }}>
                    <FileJson /> {t('Choose file')}
                  </Button>
                </div>
              </SettingRow>
              <NumberSetting section="General" name="ShufflePageCount" title="Shuffle pages" hint="How many pages of servers Shuffle looks at." min={1} max={20} fallback={5} />
              <Toggle section="General" name="UseProxies" title="Use proxies" hint="Browsers use a random proxy from proxies.txt next to the app." />
              <Toggle section="Developer" name="DisableImages" title="Disable Image Loading [Less Ram %]" />
              <Separator className="my-2" />
              <SettingRow title="App folder" hint="Accounts, settings and log.txt.">
                <Button variant="secondary" onClick={() => api.open('data')}><FolderOpen /> {t('Open')}</Button>
              </SettingRow>
              {flag('General', 'UseProxies') && <p className="py-2 text-xs text-muted-foreground">proxies.txt: [protocol://]ip:port[@user:pass]</p>}
            </TabsContent>
            </div>
          </ScrollArea>
        </Tabs>
    </Page>
  )
}
