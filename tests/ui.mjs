// UI tests and screenshots for the Rust app, driven through WebView2's DevTools protocol.
// Clicks and keys are sent to the page itself, never to the real mouse or keyboard, so nothing else on the
// desktop is touched. Runs a copy of the build in target/test-run/ui with made-up demo accounts.
//
//   node tests/ui.mjs [--release] [--screenshots <folder>] [--readme] [--keep]
//
// --readme copies the screenshots the README uses into Images/ once every test passed.

import { spawn, execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(here, '..')
const work = path.join(root, 'target', 'test-run', 'ui')
const args = process.argv.slice(2)
const shots = args.includes('--screenshots') ? path.resolve(args[args.indexOf('--screenshots') + 1]) : path.join(work, 'screenshots')
const port = 9333
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

// Anti-AFK in the test copy can never press a key: it only takes a made-up account, and waits for a day without
// input first. The real Roblox on this PC (if open) is listed but never touched.
const SAFE_ANTI_AFK = '[AntiAFK]\r\nAccounts=__test_nobody__\r\nUserSafe=true\r\nIdleSeconds=86400\r\nMaxWaitSeconds=86400\r\nKeepAwake=false\r\nAutoStart=false\r\nAutoStartIdle=false\r\nAutoRejoin=false\r\nMuteBackground=false\r\nEfficiency=false\r\nAutoGrid=false\r\n'

// ---------- demo data ----------

function prepare() {
  try { execSync(`powershell -NoProfile -Command "Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\\target\\test-run\\*' } | Stop-Process -Force"`) } catch {}
  fs.rmSync(work, { recursive: true, force: true })
  fs.mkdirSync(shots, { recursive: true })
  fs.mkdirSync(work, { recursive: true })
  const exe = path.join(root, 'target', fs.existsSync(path.join(root, 'target', 'release', 'RobloxAccountManager.exe')) && args.includes('--release') ? 'release' : 'debug', 'RobloxAccountManager.exe')
  fs.copyFileSync(exe, path.join(work, 'RobloxAccountManager.exe'))
  fs.writeFileSync(path.join(work, 'NoEncryption.IUnderstandTheRisks.iautamor'), '')
  // a test copy: its own lock and browser data, so it runs next to your own app without touching it
  fs.writeFileSync(path.join(work, 'ram-test-copy'), '')
  const now = new Date().toISOString()
  const old = new Date(Date.now() - 27 * 86400e3).toISOString()
  const demo = (Username, UserID, Group, Alias, LastUse = now, Description = '') => ({ Username, UserID, Group, Alias, LastUse, SecurityToken: `fake-token-${UserID}`, Description, Password: '', Fields: {} })
  fs.writeFileSync(path.join(work, 'AccountData.json'), JSON.stringify([
    demo('NightFalcon_07', 1, '1Main', 'Main', now, 'Main account'), demo('PixelBuilderBR', 2, '1Main', 'Builder'),
    demo('StorageAlt01', 3, '2Storage', 'Bank', old), demo('StorageAlt02', 4, '2Storage', ''),
    demo('GrindBot_A', 5, '3Farm', 'Farm A'), demo('GrindBot_B', 6, '3Farm', 'Farm B'),
  ]))
  fs.writeFileSync(path.join(work, 'RAMSettings.ini'), '[General]\r\nLanguage=en\r\nEnableMultiRbx=false\r\n' + SAFE_ANTI_AFK)
}

// ---------- DevTools protocol ----------
let socket, nextId = 1
const errors = []
const pending = new Map()

async function connect() {
  for (let i = 0; i < 60; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json()
      const page = targets.find((t) => t.type === 'page' && t.url.includes('tauri.localhost'))
      if (page) {
        socket = new WebSocket(page.webSocketDebuggerUrl)
        await new Promise((r) => socket.addEventListener('open', r))
        socket.addEventListener('message', (m) => {
          const msg = JSON.parse(m.data)
          if (msg.method === 'Runtime.exceptionThrown') errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text)
          if (msg.method === 'Runtime.consoleAPICalled' && msg.params.type === 'error') errors.push(msg.params.args.map((a) => a.value ?? a.description).join(' '))
          if (msg.id && pending.has(msg.id)) {
            pending.get(msg.id)(msg)
            pending.delete(msg.id)
          }
        })
        return
      }
    } catch {}
    await sleep(500)
  }
  throw new Error('the app window never showed up')
}

function call(method, params = {}) {
  const id = nextId++
  socket.send(JSON.stringify({ id, method, params }))
  return new Promise((resolve, reject) => pending.set(id, (m) => (m.error ? reject(new Error(m.error.message)) : resolve(m.result))))
}

async function evaluate(expression) {
  const r = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text)
  return r.result.value
}

/** Center of the first visible element matching a selector whose text (or aria-label) contains `text` */
async function find(text, selector = 'button,[role=menuitem],[role=tab],[role=option],[role=row],label') {
  return evaluate(`(() => {
    const t = ${JSON.stringify(text)}
    const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find(e => {
      const r = e.getBoundingClientRect()
      if (!(r.width > 0 && r.height > 0)) return false
      const label = (e.innerText || '').trim()
      return label === t || (e.getAttribute('aria-label') || '') === t
    }) || [...document.querySelectorAll(${JSON.stringify(selector)})].find(e => {
      const r = e.getBoundingClientRect()
      return r.width > 0 && r.height > 0 && ((e.innerText || '').trim().includes(t) || (e.getAttribute('aria-label') || '').includes(t))
    })
    if (!el) return null
    const r = el.getBoundingClientRect()
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 }
  })()`)
}

async function mouse(type, x, y, button = 'left') {
  await call('Input.dispatchMouseEvent', { type, x, y, button, clickCount: 1, buttons: button === 'left' ? 1 : 2 })
}

async function click(text, selector, button = 'left') {
  const p = await find(text, selector)
  if (!p) throw new Error(`nothing to click: "${text}"`)
  await mouse('mouseMoved', p.x, p.y, button)
  await mouse('mousePressed', p.x, p.y, button)
  await mouse('mouseReleased', p.x, p.y, button)
  await sleep(350)
}

async function key(keyName, modifiers = 0) {
  const code = keyName.length === 1 ? `Key${keyName.toUpperCase()}` : keyName
  await call('Input.dispatchKeyEvent', { type: 'keyDown', key: keyName, code, modifiers, windowsVirtualKeyCode: keyName.length === 1 ? keyName.toUpperCase().charCodeAt(0) : { Escape: 27, Enter: 13, Delete: 46 }[keyName] })
  await call('Input.dispatchKeyEvent', { type: 'keyUp', key: keyName, code, modifiers })
  await sleep(300)
}

async function shot(name) {
  const { data } = await call('Page.captureScreenshot', { format: 'png' })
  fs.writeFileSync(path.join(shots, `${name}.png`), Buffer.from(data, 'base64'))
}

const text = () => evaluate('document.body.innerText')
const rows = () => evaluate("[...document.querySelectorAll('[role=row]')].map(r => r.innerText).join(' | ')")

// ---------- tests ----------
const results = []
async function test(name, body) {
  try {
    await body()
    results.push({ name, ok: true })
    console.log(`  ok    ${name}`)
  } catch (e) {
    results.push({ name, ok: false, error: e.message })
    if (errors.length) console.log(`        page errors:\n        ${errors.splice(0).join('\n        ')}`)
    console.log(`  FAIL  ${name}: ${e.message}`)
    try { await shot(`fail-${results.length}`) } catch {}
    await key('Escape')
    await key('Escape')
  }
}

/** Waits until `check` is true: the interface has to update by itself within `ms` */
async function until(check, message, ms = 2500) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    if (await check()) return
    await sleep(100)
  }
  throw new Error(message)
}

/** Every visible enabled control shows the hand cursor, every disabled one the "not allowed" cursor */
async function checkCursors(selector) {
  const wrong = await evaluate(`(() => [...document.querySelectorAll(${JSON.stringify(selector)})]
    .filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 })
    .map((e) => {
      const data = e.getAttribute('data-disabled')
      const disabled = e.disabled || e.getAttribute('aria-disabled') === 'true' || (data !== null && data !== 'false')
      const cursor = getComputedStyle(e).cursor
      const want = disabled ? 'not-allowed' : 'pointer'
      return cursor === want ? null : (e.innerText || e.getAttribute('aria-label') || e.tagName).trim().split('\\n')[0] + ': ' + cursor + ', expected ' + want
    })
    .filter(Boolean))()`)
  assert(wrong.length === 0, `wrong cursor: ${wrong.join(' | ')}`)
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function main() {
  prepare()
  const app = spawn(path.join(work, 'RobloxAccountManager.exe'), [], {
    cwd: work,
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}` },
    stdio: 'ignore',
  })

  try {
    await connect()
    await call('Page.enable')
    await call('Runtime.enable')
    await sleep(2500)

    await test('main window lists the accounts by group', async () => {
      const t = await text()
      for (const name of ['NightFalcon_07', 'StorageAlt01', 'GrindBot_B', 'MAIN', 'STORAGE', 'FARM', '6 accounts']) assert(t.includes(name), `missing ${name}`)
      await shot('main')
    })

    await test('selecting enables the account buttons', async () => {
      await checkCursors('button, [role=row]')
      await click('NightFalcon_07', '[role=row]')
      await checkCursors('button, [role=row]')
      const disabled = await evaluate(`[...document.querySelectorAll('button')].find(b => b.innerText.includes('Remove')).disabled`)
      assert(!disabled, 'Remove still disabled')
      assert((await evaluate(`document.querySelector('input[placeholder="Alias"]').value`)) === 'Main', 'alias box')
    })

    await test('search filters and counts', async () => {
      await evaluate(`(() => { const i = document.querySelector('input[placeholder="Search accounts..."]'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, 'storage'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await sleep(300)
      assert((await text()).includes('2 of 6 accounts'), 'counter')
      await shot('search')
      await evaluate(`(() => { const i = document.querySelector('input[placeholder="Search accounts..."]'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, ''); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
    })

    await test('right click menu', async () => {
      await click('GrindBot_A', '[role=row]', 'right')
      const t = await text()
      for (const item of ['Copy', 'Groups', 'Sort Alphabetically', 'Dump Details']) assert(t.includes(item), `menu misses ${item}`)
      await checkCursors('[role=menuitem]')
      await shot('account-menu')
      await key('Escape')
    })

    await test('colors for an account and a group, from the right click menus', async () => {
      const style = (expression) => evaluate(`(() => { const e = ${expression}; return e ? getComputedStyle(e).color + '|' + (e.closest('[role=row]')?.querySelector('span[aria-hidden]')?.style.background ?? '') : null })()`)
      const name = `[...document.querySelectorAll('[role=row] span.font-medium')].find((e) => e.innerText === 'GrindBot_A')`
      const header = `[...document.querySelectorAll('button')].find((e) => e.innerText.startsWith('FARM'))`

      // one account in red
      await click('GrindBot_A', '[role=row]', 'right')
      await click('Color', '[role=menuitem]')
      await checkCursors('[role=menuitem]')
      await shot('color-menu')
      await click('Red', '[role=menuitem]')
      await until(async () => (await style(name))?.startsWith('rgb(239, 68, 68)|rgb(239, 68, 68)'), 'the account is not red')

      // its group in blue: the header, and the bar of the group's other account
      await click('FARM', 'button', 'right')
      await click('Group color', '[role=menuitem]')
      await click('Blue', '[role=menuitem]')
      await until(async () => (await style(header))?.startsWith('rgb(59, 130, 246)'), 'the group is not blue')
      const other = `[...document.querySelectorAll('[role=row] span.font-medium')].find((e) => e.innerText === 'GrindBot_B')`
      await until(async () => (await style(other))?.endsWith('rgb(59, 130, 246)'), "the group's color is not on its accounts")
      // the account's own color wins over its group's
      assert((await style(name)).endsWith('rgb(239, 68, 68)'), "the account's color lost to the group's")
      await shot('colors')

      // and back to no colors
      await click('GrindBot_A', '[role=row]', 'right')
      await click('Color', '[role=menuitem]')
      await click('No color', '[role=menuitem]')
      await click('FARM', 'button', 'right')
      await click('Group color', '[role=menuitem]')
      await click('No color', '[role=menuitem]')
      await until(async () => (await style(other))?.endsWith('|'), 'the colors did not go away')
    })

    await test('remove shows undo and undo brings it back', async () => {
      await click('StorageAlt02', '[role=row]')
      await click('Remove', 'button')
      await sleep(500)
      assert(!(await rows()).includes('StorageAlt02'), 'not removed')
      await click('Undo', 'button')
      await sleep(800)
      assert((await rows()).includes('StorageAlt02'), 'undo failed')
    })

    await test('command palette (Ctrl+K)', async () => {
      await key('k', 2)
      assert((await text()).includes('Search accounts and actions') || (await find('Join Server', '[role=option]')), 'palette not open')
      // account actions follow the selection, and the cursor says which items are usable
      await checkCursors('[role=option]')
      await shot('palette')
      await key('Escape')
    })

    await test('settings tabs', async () => {
      await click('Settings', 'button')
      await shot('settings-general')
      await click('Developer', '[role=tab]')
      await shot('settings-developer')
      await click('Miscellaneous', '[role=tab]')
      await shot('settings-misc')
      await key('Escape')
    })

    await test('automatic updates can be turned off with one choice', async () => {
      const ini = () => fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')
      const updates = () => /\[Updates\][^[]*/.exec(ini())?.[0] ?? ''
      await click('Settings', 'button')
      await evaluate(`[...document.querySelectorAll('[data-slot=setting-row], div')].find((e) => e.innerText?.startsWith('Automatic updates'))?.scrollIntoView()`)
      await click('Automatic', '[role=combobox]')
      await click('Off', '[role=option]')
      await until(async () => /Check=false/.test(updates()) && /AutoInstall=false/.test(updates()), `not saved as off:\n${updates()}`)
      assert((await text()).includes('never contacts GitHub'), 'no explanation for "Off"')
      await click('Off', '[role=combobox]')
      await click('Automatic', '[role=option]')
      await until(async () => /Check=true/.test(updates()) && /AutoInstall=true/.test(updates()), `not saved as automatic:\n${updates()}`)
      await key('Escape')
    })

    await test('theme editor', async () => {
      await click('Theme Editor', 'button')
      assert((await text()).includes('Midnight'), 'presets')
      await shot('theme')
      await key('Escape')
    })

    await test('anti-afk window', async () => {
      await click('Anti-AFK', 'button')
      assert((await text()).includes('Keeps your Roblox windows'), 'not open')
      await shot('antiafk-windows')
      for (const tab of ['Behaviour', 'Rejoin & extras', 'Discord', 'Statistics']) {
        await click(tab, '[role=tab]')
        await shot(`antiafk-${tab.split(' ')[0].toLowerCase()}`)
      }
      await key('Escape')
    })

    await test('anti-afk start and stop update by themselves', async () => {
      await click('Anti-AFK', 'button')
      const windowsOpen = !(await text()).includes('0 Roblox windows open')
      await click('Start', 'button')
      await until(async () => (await text()).includes('Running') && (await find('Stop', 'button')) !== null, 'Start did not change to Running / Stop')

      if (windowsOpen) {
        // the countdown moves on its own, every second
        const line = async () => /Next action in \d+:\d+/.exec(await text())?.[0]
        const first = await line()
        assert(first, 'no countdown')
        await until(async () => (await line()) !== first, 'the countdown is frozen')
        // once the round is due it waits for the user to stop (here for a day), and says so
        await until(async () => (await text()).includes('Waiting for you to stop typing and moving the mouse'), 'no waiting state', 8000)
      } else {
        await until(async () => (await text()).includes('Waiting for a Roblox window'), 'no "waiting for a Roblox window" state')
      }

      await click('Stop', 'button')
      await until(async () => (await text()).includes('Stopped') && (await find('Start', 'button')) !== null, 'Stop did not change to Stopped / Start')
      await key('Escape')
    })

    await test('utilities tabs', async () => {
      await click('Utilities', 'button')
      for (const tab of ['Servers', 'Favorites', 'Universe', 'Outfits', 'Watcher']) {
        await click(tab, '[role=tab]')
        await shot(`utilities-${tab.toLowerCase()}`)
      }
      await key('Escape')
    })

    await test('account utilities', async () => {
      await click('NightFalcon_07', '[role=row]')
      await click('Account Utilities', 'button')
      assert((await text()).includes('Change Password'), 'not open')
      await shot('account-utilities')
      await key('Escape')
    })

    await test('language switch is instant', async () => {
      await click('EN', 'button')
      await click('Português (Portugal)', '[role=menuitem]')
      await sleep(500)
      assert((await text()).includes('Adicionar conta'), 'not translated')
      await shot('main-pt')
      for (const [code, name, word] of [['PT', 'Español', 'Añadir cuenta'], ['ES', 'Français', 'Ajouter un compte'], ['FR', 'Deutsch', 'Konto hinzufügen']]) {
        await click(code, 'button')
        await click(name, '[role=menuitem]')
        await sleep(500)
        assert((await text()).includes(word), `${name} not translated`)
        await shot(`main-${name.slice(0, 2).toLowerCase()}`)
      }
      await click('DE', 'button')
      await click('English', '[role=menuitem]')
    })
  } finally {
    if (!args.includes('--keep')) app.kill()
  }

  const failed = results.filter((r) => !r.ok)
  console.log(`\n${results.length - failed.length} passed, ${failed.length} failed. Screenshots: ${shots}`)
  if (!failed.length && args.includes('--readme')) updateReadmeImages()
  process.exit(failed.length ? 1 : 0)
}

/** The README's pictures. Not the Anti-AFK "Windows" tab: it lists the real Roblox windows open on this PC. */
function updateReadmeImages() {
  const pictures = {
    main: 'main', 'main-pt': 'main-pt', 'main-es': 'main-es', 'main-fr': 'main-fr', 'main-de': 'main-de',
    palette: 'command-palette', 'antiafk-behaviour': 'antiafk', 'antiafk-rejoin': 'antiafk-rejoin',
    search: 'search', 'account-menu': 'account-menu', 'account-utilities': 'account-utilities',
    'settings-general': 'settings', theme: 'theme', 'utilities-servers': 'utilities',
  }
  const images = path.join(root, 'Images')
  fs.mkdirSync(images, { recursive: true })
  for (const [shot, name] of Object.entries(pictures)) fs.copyFileSync(path.join(shots, `${shot}.png`), path.join(images, `${name}.png`))
  console.log(`README images updated in ${images}`)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
