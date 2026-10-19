/** Anti-AFK: status and windows, tools, how it behaves, rejoining, Discord and statistics */

import { Bell, Eye, EyeOff, Gauge, History, LayoutGrid, Play, Power, RotateCcw, Send, Square, Trash2, Volume2, VolumeX, X, Zap } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { NumberSetting, SettingRow, TextSetting, Toggle } from '@/components/common/SettingRow'
import { useDialogs } from '@/components/dialogs/DialogHost'
import { clock, duration, useAntiAfk } from '@/hooks/useAntiAfk'
import { api } from '@/lib/api'
import { useI18n, useT } from '@/lib/i18n'
import { notifier } from '@/lib/notify'
import { useStore } from '@/lib/store'
import type { PastStats } from '@/lib/types'

/** Minutes in the box, seconds in RAMSettings.ini; from half a minute to 19 (Roblox kicks after 20) */
function MinutesInput({ name, fallback }: { name: string; fallback: number }) {
  const { setting, updateSetting } = useStore()
  const seconds = Number(setting('AntiAFK', name)) || fallback
  return (
    <Input
      type="number"
      className="w-20"
      min={0.5}
      max={19}
      step={0.5}
      defaultValue={Math.round((seconds / 60) * 10) / 10}
      key={seconds}
      onBlur={(e) => {
        const minutes = Math.min(19, Math.max(0.5, Number(e.target.value.replace(',', '.')) || fallback / 60))
        const next = Math.round(minutes * 60)
        e.target.value = String(Math.round((next / 60) * 10) / 10)
        if (next !== seconds) void updateSetting('AntiAFK', name, String(next))
      }}
      onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
    />
  )
}

function Stat({ label, value }: { label: string; value: string | number }) {
  const t = useT()
  return (
    <div className="rounded-lg border bg-background/40 p-3">
      <div className="text-xs text-muted-foreground">{t(label)}</div>
      <div className="mt-1 text-lg font-semibold tabular-nums">{value}</div>
    </div>
  )
}

/** One reset's statistics, with the dates they cover and a delete button */
function PastStatsCard({ past, onDelete }: { past: PastStats; onDelete: () => void }) {
  const { t, language } = useI18n()
  const date = (seconds: number) => new Date(seconds * 1000).toLocaleDateString(language, { day: 'numeric', month: 'short', year: 'numeric' })
  const range = past.since ? `${date(past.since)} – ${date(past.until)}` : t('Until {0}', date(past.until))
  const items: [string, string | number][] = [
    ['Time kept active', duration(past.seconds)],
    ['Longest session', duration(past.longest)],
    ['Sessions', past.sessions],
    ['Key press rounds', past.actions],
    ['Rejoins', past.rejoins],
  ]
  return (
    <div className="flex items-start gap-3 rounded-lg border bg-background/40 p-3" data-testid="past-stats">
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{range}</div>
        <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          {items.map(([label, value]) => (
            <span key={label}>
              {t(label)}: <span className="font-medium text-foreground tabular-nums">{value}</span>
            </span>
          ))}
        </div>
      </div>
      <Button variant="ghost" size="icon" className="shrink-0" aria-label={t('Delete')} onClick={onDelete}>
        <Trash2 />
      </Button>
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
  const randomInterval = setting('AntiAFK', 'IntervalMode') === 'random'

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
  // statistics kept from earlier resets
  const [history, setHistory] = useState<PastStats[]>([])
  useEffect(() => {
    if (panel === 'antiafk') api.antiAfkStatsHistory().then(setHistory, () => {})
  }, [panel])
  const reset = async () => {
    if (!(await dialogs.confirm({ title: 'Reset the statistics?', description: 'They start again from zero. The current ones are kept in the history below, where you can delete them.' }))) return
    await api.antiAfkResetStats()
    setHistory(await api.antiAfkStatsHistory())
  }
  const deletePast = async (id: number) => {
    if (!(await dialogs.confirm({ title: 'Delete these statistics?', destructive: true }))) return
    setHistory(await api.antiAfkDeletePastStats(id))
  }

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
            <div className="font-semibold">{running ? t('Running') : t('Stopped')}</div>
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
            {running ? <Square /> : <Play />} {running ? t('Stop') : t('Start')}
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
                    <Badge variant={w.state === 'disconnected' ? 'destructive' : 'secondary'}>{t(w.state === 'playing' ? 'In game' : w.state === 'disconnected' ? 'Disconnected' : 'Loading')}</Badge>
                  </label>
                ))}
                {status?.windows.length === 0 && <p className="py-8 text-center text-sm text-muted-foreground">{t('No Roblox windows open. Join a game and they show here.')}</p>}
              </div>
            </TabsContent>

            <TabsContent value="behaviour" className="divide-y">
              <SettingRow title="Interval" hint="How often each window gets a key press. Roblox kicks after 20 minutes.">
                <Select value={randomInterval ? 'random' : 'fixed'} onValueChange={(v) => updateSetting('AntiAFK', 'IntervalMode', v)}>
                  <SelectTrigger className="w-36"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem value="fixed">{t('Fixed')}</SelectItem>
                    <SelectItem value="random">{t('Random')}</SelectItem>
                  </SelectContent>
                </Select>
              </SettingRow>
              {randomInterval ? (
                <SettingRow title="Between (minutes)" hint="Each round waits a different time between these two, so it's less regular.">
                  <div className="flex items-center gap-2">
                    <MinutesInput name="IntervalMin" fallback={360} />
                    <span className="text-sm text-muted-foreground">{t('and')}</span>
                    <MinutesInput name="IntervalMax" fallback={900} />
                  </div>
                </SettingRow>
              ) : (
                <SettingRow title="Every (minutes)" hint="From half a minute to 19.">
                  <MinutesInput name="Interval" fallback={540} />
                </SettingRow>
              )}
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
              <Toggle section="AntiAFK" name="SkipFullScreen" title="Wait while a full screen app is in front" hint="Games, videos and presentations in full screen are never interrupted." />
              <Toggle section="AntiAFK" name="AutoStart" title="Start when Roblox opens" />
              <Toggle section="AntiAFK" name="AutoStartIdle" title="Start when I'm away" hint="After 19 minutes without input, with a reminder at 18." />
              <Toggle section="AntiAFK" name="KeepAwake" title="Keep the PC awake while running" />
              <Toggle section="General" name="CloseToTray" title="Keep running in the tray when the window is closed" />
            </TabsContent>

            <TabsContent value="rejoin" className="divide-y">
              <Toggle section="AntiAFK" name="AutoRejoin" title="Rejoin disconnected accounts" hint="Reads the Roblox log, closes the disconnected window and launches the account into the same game again." />
              <Toggle section="AntiAFK" name="RejoinSameServer" title="Rejoin the same server" hint="If it closed (for example for a game update) or is full, any server of the game. Off: always any server." disabled={!flag('AntiAFK', 'AutoRejoin')} />
              <Toggle section="AntiAFK" name="AutoReset" title="Reset the character every round" hint="Esc, R, Enter after the key presses." />
              <Toggle section="AntiAFK" name="MuteBackground" title="Mute the windows I'm not looking at" hint="The window in front keeps its sound." />
              <Toggle section="AntiAFK" name="Efficiency" title="Efficiency mode for background windows" hint="Windows' efficiency mode and a lower priority: less CPU and fan noise with many accounts. Safe, Roblox is never paused." />
              <Toggle section="AntiAFK" name="AutoGrid" title="Arrange windows automatically" hint="A grid every time a Roblox window opens or closes." />
              <div className="flex items-start gap-2 py-3 text-xs text-muted-foreground"><Gauge className="size-4 shrink-0" />{t('To make an account use less CPU, right click it > CPU limit.')}</div>
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
              <Button variant="outline" className="justify-self-start" onClick={reset}>
                <RotateCcw /> {t('Reset statistics')}
              </Button>
              {history.length > 0 && (
                <section className="grid gap-2 pt-2">
                  <div className="flex items-center gap-2 text-sm font-medium"><History className="size-4" /> {t('History')}</div>
                  {history.map((past) => (
                    <PastStatsCard key={past.id} past={past} onDelete={() => void deletePast(past.id)} />
                  ))}
                </section>
              )}
            </TabsContent>
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
