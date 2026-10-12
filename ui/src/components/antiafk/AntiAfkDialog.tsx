/** Anti-AFK: status and windows, tools, how it behaves, rejoining, Discord and statistics */

import { Bell, Eye, EyeOff, Gauge, LayoutGrid, Play, Power, RotateCcw, Send, Square, Volume2, VolumeX, X, Zap } from 'lucide-react'
import { useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { NumberSetting, SettingRow, TextSetting, Toggle } from '@/components/common/SettingRow'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { clock, duration, useAntiAfk } from '@/hooks/useAntiAfk'
import { api } from '@/lib/api'
import { useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'

const INTERVALS = [3, 6, 9, 12, 15, 18]

function Stat({ label, value }: { label: string; value: string | number }) {
  const t = useT()
  return (
    <div className="rounded-lg border bg-background/40 p-3">
      <div className="text-xs text-muted-foreground">{t(label)}</div>
      <div className="mt-1 text-lg font-semibold tabular-nums">{value}</div>
    </div>
  )
}

export function AntiAfkDialog() {
  const t = useT()
  const notify = notifier(t)
  const dialogs = useDialogs()
  const status = useAntiAfk()
  const { panel, setPanel, setting, updateSetting, flag } = useStore()
  const included = setting('AntiAFK', 'Accounts').split(',').map((s) => s.trim()).filter(Boolean)
  const interval = Number(setting('AntiAFK', 'Interval') || 540)

  const tool = async (kind: Parameters<typeof api.antiAfkTool>[0]) => {
    try {
      notify.success(await api.antiAfkTool(kind))
    } catch (e) {
      notify.error(e)
    }
  }

  const toggleAccount = (name: string, on: boolean) => {
    // an empty list means every account, the first unticked one starts the list with all the others
    const all = status?.windows.map((w) => w.account).filter((a): a is string => !!a) ?? []
    const base = included.length ? included : all
    const next = on ? [...new Set([...base, name])] : base.filter((a) => a !== name)
    void updateSetting('AntiAFK', 'Accounts', next.length === all.length ? '' : next.join(','))
  }

  const running = status?.running ?? false
  // the window tools need a Roblox window (showing hidden ones always works)
  const noWindows = !status?.windows.length
  const [sending, setSending] = useState(false)

  return (
    <Dialog open={panel === 'antiafk'} onOpenChange={(open) => !open && setPanel(null)}>
      <DialogContent className="flex h-[85vh] flex-col sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t('Anti-AFK')}</DialogTitle>
          <DialogDescription>{t('Keeps your Roblox windows from being kicked for being idle (20 minutes), without getting in your way.')}</DialogDescription>
        </DialogHeader>

        <div className={`flex items-center gap-4 rounded-xl border p-4 ${running ? 'border-emerald-500/40 bg-emerald-500/5' : ''}`}>
          <div className={`grid size-12 place-items-center rounded-full ${running ? 'bg-emerald-500/15 text-emerald-500' : 'bg-muted text-muted-foreground'}`}>
            <Power />
          </div>
          <div className="flex-1">
            <div className="font-semibold">{t(running ? 'Running' : 'Stopped')}</div>
            <div className="text-sm text-muted-foreground">
              {running
                ? status?.phase === 'acting'
                  ? t('Pressing keys in the Roblox windows…')
                  : status?.phase === 'waiting'
                    ? t('Waiting for you to stop typing and moving the mouse')
                    : status?.secondsToNext != null
                      ? t('Next action in {0}', clock(status.secondsToNext))
                      : t('Waiting for a Roblox window')
                : status?.windows.length === 1
                  ? t('1 Roblox window open')
                  : t('{0} Roblox windows open', status?.windows.length ?? 0)}
            </div>
          </div>
          {running && (
            <Button variant="secondary" disabled={status?.phase === 'acting'} onClick={() => api.antiAfkNow()}>
              <Zap /> {t('Run now')}
            </Button>
          )}
          <Button size="lg" variant={running ? 'secondary' : 'default'} onClick={() => (running ? api.antiAfkStop() : api.antiAfkStart())}>
            {running ? <Square /> : <Play />} {t(running ? 'Stop' : 'Start')}
          </Button>
        </div>

        <Tabs defaultValue="windows" className="flex min-h-0 flex-1 flex-col">
          <TabsList>
            <TabsTrigger value="windows">{t('Windows')}</TabsTrigger>
            <TabsTrigger value="behaviour">{t('Behaviour')}</TabsTrigger>
            <TabsTrigger value="rejoin">{t('Rejoin & extras')}</TabsTrigger>
            <TabsTrigger value="discord">Discord</TabsTrigger>
            <TabsTrigger value="stats">{t('Statistics')}</TabsTrigger>
          </TabsList>

          <ScrollArea className="mt-3 min-h-0 flex-1 pr-3">
            <TabsContent value="windows" className="grid gap-3">
              <div className="flex flex-wrap gap-2">
                <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('grid')}><LayoutGrid /> {t('Arrange in a grid')}</Button>
                {status?.hidden ? (
                  <Button variant="secondary" size="sm" onClick={() => tool('show')}><Eye /> {t('Show windows')}</Button>
                ) : (
                  <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('hide')}><EyeOff /> {t('Hide windows')}</Button>
                )}
                <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('seeThrough')}>{t('See-through')}</Button>
                <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('solid')}>{t('Solid')}</Button>
                <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('mute')}><VolumeX /> {t('Mute all')}</Button>
                <Button variant="secondary" size="sm" disabled={noWindows} onClick={() => tool('unmute')}><Volume2 /> {t('Unmute all')}</Button>
                <Button variant="ghost" size="sm" className="text-destructive" disabled={noWindows} onClick={async () => { if (await dialogs.confirm({ title: 'Close every Roblox window?', destructive: true })) await tool('closeAll') }}>
                  <X /> {t('Close all')}
                </Button>
              </div>

              <div className="grid gap-1.5">
                {(status?.windows ?? []).map((w) => (
                  <label key={w.pid} className="flex items-center gap-3 rounded-lg border p-3">
                    <Checkbox checked={w.included} disabled={!w.account} onCheckedChange={(on) => w.account && toggleAccount(w.account, on === true)} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate font-medium">{w.account ?? t('Roblox (not opened by the app)')}</div>
                      <div className="text-xs text-muted-foreground">PID {w.pid}{w.placeId ? ` · ${t('Place {0}', w.placeId)}` : ''}</div>
                    </div>
                    {w.background && <Badge variant="outline">{t('In the background')}</Badge>}
                    <Badge variant={w.state === 'disconnected' ? 'destructive' : 'secondary'}>{t(w.state === 'playing' ? 'In game' : w.state === 'disconnected' ? 'Disconnected' : 'Loading')}</Badge>
                  </label>
                ))}
                {status?.windows.length === 0 && <p className="py-8 text-center text-sm text-muted-foreground">{t('No Roblox windows open. Join a game and they show here.')}</p>}
              </div>
            </TabsContent>

            <TabsContent value="behaviour" className="divide-y">
              <SettingRow title="Interval" hint="How often each window gets a key press. Roblox kicks after 20 minutes.">
                <Select value={String(interval)} onValueChange={(v) => updateSetting('AntiAFK', 'Interval', v)}>
                  <SelectTrigger className="w-36"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {INTERVALS.map((m) => <SelectItem key={m} value={String(m * 60)}>{t('{0} minutes', m)}</SelectItem>)}
                    {!INTERVALS.includes(interval / 60) && <SelectItem value={String(interval)}>{t('{0} seconds', interval)}</SelectItem>}
                  </SelectContent>
                </Select>
              </SettingRow>
              <SettingRow title="Action" hint="What is pressed in each window.">
                <Select value={setting('AntiAFK', 'Action') || 'space'} onValueChange={(v) => updateSetting('AntiAFK', 'Action', v)}>
                  <SelectTrigger className="w-44"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="space">{t('Jump (Space)')}</SelectItem>
                    <SelectItem value="ws">{t('Step (W and S)')}</SelectItem>
                    <SelectItem value="zoom">{t('Zoom (I and O)')}</SelectItem>
                    <SelectItem value="camera">{t('Turn the camera')}</SelectItem>
                  </SelectContent>
                </Select>
              </SettingRow>
              <NumberSetting section="AntiAFK" name="Repeat" title="Presses per window" min={1} max={10} fallback={3} />
              <Toggle section="AntiAFK" name="UserSafe" title="Don't interrupt me" hint="Waits until you stop typing and moving the mouse before switching windows." />
              <NumberSetting section="AntiAFK" name="IdleSeconds" title="Seconds without input first" min={1} max={120} fallback={5} />
              <NumberSetting section="AntiAFK" name="MaxWaitSeconds" title="Give up waiting after (s)" hint="Then it tries again 30 seconds later instead of interrupting you." min={10} max={600} fallback={90} />
              <Toggle section="AntiAFK" name="SkipFocused" title="Skip the window I'm playing in" hint="A Roblox window you used in the last minute isn't idle." />
              <Toggle section="AntiAFK" name="Invisible" title="Invisible while pressing keys" hint="The Roblox window turns see-through for that moment, so nothing pops up in front of you." />
              <Toggle section="AntiAFK" name="SkipFullScreen" title="Wait while a full screen app is in front" hint="Games, videos and presentations in full screen are never interrupted." />
              <Toggle section="AntiAFK" name="Background" title="Background only (experimental)" hint="Never switches windows: the keys go straight to Roblox. Some games ignore them; if an account gets kicked for being idle, it goes back to the normal mode by itself." />
              {setting('AntiAFK', 'NoBackground') && (
                <SettingRow title="Normal mode for" hint={setting('AntiAFK', 'NoBackground').split(',').join(', ')}>
                  <Button variant="secondary" onClick={() => updateSetting('AntiAFK', 'NoBackground', '')}>{t('Try the background again')}</Button>
                </SettingRow>
              )}
              <Toggle section="AntiAFK" name="AutoStart" title="Start when Roblox opens" />
              <Toggle section="AntiAFK" name="AutoStartIdle" title="Start when I'm away" hint="After 19 minutes without input, with a reminder at 18." />
              <Toggle section="AntiAFK" name="KeepAwake" title="Keep the PC awake while running" />
              <Toggle section="General" name="CloseToTray" title="Keep running in the tray when the window is closed" />
            </TabsContent>

            <TabsContent value="rejoin" className="divide-y">
              <Toggle section="AntiAFK" name="AutoRejoin" title="Rejoin disconnected accounts" hint="Reads the Roblox log, closes the disconnected window and launches the account into the same game again." />
              <Toggle section="AntiAFK" name="RejoinSameServer" title="Rejoin the same server" hint="Otherwise any server of the game." disabled={!flag('AntiAFK', 'AutoRejoin')} />
              <Toggle section="AntiAFK" name="AutoReset" title="Reset the character every round" hint="Esc, R, Enter after the key presses." />
              <Toggle section="AntiAFK" name="MuteBackground" title="Mute the windows I'm not looking at" hint="The window in front keeps its sound." />
              <Toggle section="AntiAFK" name="Efficiency" title="Efficiency mode for background windows" hint="Windows' efficiency mode and a lower priority: less CPU and fan noise with many accounts. Safe, Roblox is never paused." />
              <Toggle section="AntiAFK" name="AutoGrid" title="Arrange windows automatically" hint="A grid every time a Roblox window opens or closes." />
              <div className="flex items-start gap-2 py-3 text-xs text-muted-foreground"><Gauge className="size-4 shrink-0" />{t('For an FPS limit use Settings > Miscellaneous > Max FPS, Roblox applies it on the next launch.')}</div>
            </TabsContent>

            <TabsContent value="discord" className="divide-y">
              <TextSetting section="AntiAFK" name="Webhook" title="Discord webhook" hint="Server Settings > Integrations > Webhooks > Copy URL" placeholder="https://discord.com/api/webhooks/…" password width="w-72" />
              <SettingRow title="Send a test message">
                <Button variant="secondary" disabled={!setting('AntiAFK', 'Webhook') || sending} onClick={async () => { setSending(true); try { await api.antiAfkTestWebhook(); notify.success('Sent') } catch (e) { notify.error(e) } finally { setSending(false) } }}>
                  <Send /> {t('Test')}
                </Button>
              </SettingRow>
              <div className="flex items-center gap-2 py-3 text-sm font-medium"><Bell className="size-4" /> {t('Notify when')}</div>
              <Toggle section="AntiAFK" name="WebhookOnStart" title="It starts" />
              <Toggle section="AntiAFK" name="WebhookOnStop" title="It stops" />
              <Toggle section="AntiAFK" name="WebhookOnAction" title="Every round" />
              <Toggle section="AntiAFK" name="WebhookOnRejoin" title="An account disconnects or rejoins" />
              <Toggle section="AntiAFK" name="WebhookOnError" title="Something fails" />
            </TabsContent>

            <TabsContent value="stats" className="grid gap-3">
              <div className="grid grid-cols-3 gap-2">
                <Stat label="Time kept active" value={duration(status?.stats.seconds ?? 0)} />
                <Stat label="Longest session" value={duration(status?.stats.longest ?? 0)} />
                <Stat label="Sessions" value={status?.stats.sessions ?? 0} />
                <Stat label="Key press rounds" value={status?.stats.actions ?? 0} />
                <Stat label="Rejoins" value={status?.stats.rejoins ?? 0} />
                <Stat label="You're idle for" value={clock(status?.idleSeconds ?? 0)} />
              </div>
              <Button variant="outline" className="justify-self-start" onClick={async () => { if (await dialogs.confirm({ title: 'Reset the statistics?', destructive: true })) await api.antiAfkResetStats() }}>
                <RotateCcw /> {t('Reset statistics')}
              </Button>
            </TabsContent>
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
