/** Settings: General, Developer and Miscellaneous, every change applies right away */

import { FileJson, FolderOpen, Lock } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Flag } from '@/components/common/Flag'
import { NumberSetting, SettingRow, TextSetting, Toggle } from '@/components/common/SettingRow'
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

export function SettingsDialog() {
  const { t, language, setTable } = useI18n()
  const { panel, setPanel, boot, setting, updateSetting, flag } = useStore()
  const [startup, setStartup] = useState(boot.startup)
  const changeProtection = useProtectionChange()
  const customSettings = setting('General', 'CustomClientSettings')

  const changeLanguage = async (code: string) => {
    const result = await api.setLanguage(code)
    setTable(result.language, result.table)
  }

  return (
    <Dialog open={panel === 'settings'} onOpenChange={(open) => !open && setPanel(null)}>
      <DialogContent className="flex max-h-[85vh] flex-col sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t('Settings')}</DialogTitle>
          <DialogDescription>{t('Changes are saved and applied right away.')}</DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="general" className="min-h-0 flex-1">
          <TabsList>
            <TabsTrigger value="general">{t('General')}</TabsTrigger>
            <TabsTrigger value="developer">{t('Developer')}</TabsTrigger>
            <TabsTrigger value="misc">{t('Miscellaneous')}</TabsTrigger>
          </TabsList>

          <ScrollArea className="mt-2 h-[52vh] pr-3">
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
              <Toggle section="General" name="AsyncJoin" title="Async Launching" hint="When launching several accounts, start the next one as soon as the previous opened instead of waiting the delay." />
              <NumberSetting section="General" name="AccountJoinDelay" title="Launch Delay" hint="Seconds between accounts when launching several." min={0} max={120} fallback={8} />
              <Toggle section="General" name="SavePasswords" title="Save Passwords" hint="Remember the password typed when logging in, to copy it later." />
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
              <Toggle section="General" name="UnlockFPS" title="Unlock FPS" hint="Raise Roblox's 60 FPS cap (ClientAppSettings.json)." disabled={!!customSettings} />
              <NumberSetting section="General" name="MaxFPSValue" title="Max FPS" min={30} max={1000} fallback={120} />
              <SettingRow title="Use Custom ClientAppSettings" hint={customSettings || 'Copy your own ClientAppSettings.json into Roblox before every launch.'}>
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
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
