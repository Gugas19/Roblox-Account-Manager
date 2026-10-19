// UI tests and screenshots for the Rust app, driven through WebView2's DevTools protocol.
// Clicks and keys are sent to the page itself, never to the real mouse or keyboard, so nothing else on the
// desktop is touched. Runs a copy of the build in target/test-run/ui-<time> with made-up demo accounts; the
// screenshots go to target/test-run/ui-screenshots.
//
//   node tests/ui.mjs [--release] [--screenshots <folder>] [--readme] [--keep] [--only <part of a test name>]
//
// --readme copies the screenshots the README uses into Images/ once every test passed.

import { spawn, execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(here, '..')
// a folder of its own each run: the last run's WebView2 (or the antivirus) can hold its files for a while, and
// removing them made the tests fail; old run folders are tidied up when they let go
const runs = path.join(root, 'target', 'test-run')
const work = path.join(runs, `ui-${Date.now()}`)
const args = process.argv.slice(2)
const shots = args.includes('--screenshots') ? path.resolve(args[args.indexOf('--screenshots') + 1]) : path.join(runs, 'ui-screenshots')
const port = 9333
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

// Anti-AFK in the test copy can never press a key: it only takes a made-up account, and waits for a day without
// input first. The real Roblox on this PC (if open) is listed but never touched.
const SAFE_ANTI_AFK = '[AntiAFK]\r\nAccounts=__test_nobody__\r\nUserSafe=true\r\nIdleSeconds=86400\r\nMaxWaitSeconds=86400\r\nKeepAwake=false\r\nAutoStart=false\r\nAutoStartIdle=false\r\nAutoRejoin=false\r\nAutoRejoinDefaultOn=true\r\nMuteBackground=false\r\nEfficiency=false\r\nAutoGrid=false\r\nStatSeconds=7200\r\nStatActions=30\r\nStatRejoins=1\r\nStatSessions=3\r\nStatLongest=3600\r\nStatSince=1760000000\r\n'

// ---------- demo data ----------

function prepare() {
  // the copy and its browser (WebView2 outlives the app by a few seconds and holds the folder); nothing else
  try { execSync(`powershell -NoProfile -Command "Get-CimInstance Win32_Process | Where-Object { ($_.Name -eq 'RobloxAccountManager.exe' -and $_.ExecutablePath -like '*\\target\\test-run\\*') -or ($_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -like '*\\target\\test-run\\*') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"`) } catch {}
  // a just closed copy (or the antivirus) can hold its files a moment
  for (const old of fs.existsSync(runs) ? fs.readdirSync(runs).filter((n) => /^ui(-\d+)?$/.test(n)) : []) {
    try { fs.rmSync(path.join(runs, old), { recursive: true, force: true }) } catch {}
  }
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
    demo('MainAccount', 1, '1Main', 'Main', now, 'Main account'), demo('BuilderAccount', 2, '1Main', 'Builder'),
    demo('StorageAlt01', 3, '2Storage', 'Bank', old), demo('StorageAlt02', 4, '2Storage', ''),
    demo('GrindBot_A', 5, '3Farm', 'Farm A'), { ...demo('GrindBot_B', 6, '3Farm', 'Farm B'), Password: 'saved-demo-password' },
  ]))
  // the README's pictures show the normal settings. Still safe: Anti-AFK only takes a made-up account, so it never
  // finds a window of its own to switch to or press keys in
  const settings = args.includes('--readme')
    ? '[General]\r\nLanguage=en\r\n[Updates]\r\nChosen=true\r\n' + SAFE_ANTI_AFK.replace('IdleSeconds=86400', 'IdleSeconds=5').replace('MaxWaitSeconds=86400', 'MaxWaitSeconds=90').replace('AutoRejoin=false', 'AutoRejoin=true')
    : '[General]\r\nLanguage=en\r\nEnableMultiRbx=false\r\n[Updates]\r\nChosen=true\r\n' + SAFE_ANTI_AFK
  fs.writeFileSync(path.join(work, 'RAMSettings.ini'), settings)
  // a made-up session history: two accounts, yesterday and today
  const at = (hoursAgo) => Math.floor(Date.now() / 1000) - hoursAgo * 3600
  fs.writeFileSync(path.join(work, 'SessionHistory.json'), JSON.stringify([
    { at: at(26), account: 'MainAccount', kind: 'online' },
    { at: at(25), account: 'MainAccount', kind: 'joined', placeId: 1818, place: 'Demo Game', jobId: '11111111-2222-3333-4444-555555555555' },
    { at: at(3), account: 'GrindBot_A', kind: 'disconnected', placeId: 1818, place: 'Demo Game' },
    { at: at(3), account: 'GrindBot_A', kind: 'rejoined', placeId: 1818, place: 'Demo Game' },
    { at: at(1), account: 'MainAccount', kind: 'offline' },
  ]))
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

/** Opens a right click submenu, clicking its trigger again if the menu was still opening */
async function openSub(text) {
  for (let attempt = 0; attempt < 4; attempt++) {
    await click(text, '[role=menuitem]')
    const open = await evaluate(`!!document.querySelector('[data-slot=context-menu-sub-content][data-state=open]')`)
    if (open) return
    await sleep(100)
  }
  throw new Error(`the "${text}" submenu didn't open`)
}

/** Right click until the menu shows the item (a busy PC can miss the first one), then clicks the item */
async function menuItem(target, targetSelector, item) {
  for (let attempt = 0; attempt < 4; attempt++) {
    await click(target, targetSelector, 'right')
    if (await findSoon(item, '[role=menuitem]')) return click(item, '[role=menuitem]')
    await key('Escape')
  }
  throw new Error(`the menu of "${target}" never showed "${item}"`)
}

/** Menus and dialogs animate for 100 ms: after an action, a short settle before the next one */
const SETTLE = 150

/** Waits (up to 3 s) for the element to show up, so no test needs a long fixed pause before a click */
async function findSoon(text, selector) {
  const end = Date.now() + 3000
  for (;;) {
    const p = await find(text, selector)
    if (p || Date.now() > end) return p
    await sleep(50)
  }
}

async function click(text, selector, button = 'left') {
  const p = await findSoon(text, selector)
  if (!p) throw new Error(`nothing to click: "${text}"`)
  await mouse('mouseMoved', p.x, p.y, button)
  await mouse('mousePressed', p.x, p.y, button)
  await mouse('mouseReleased', p.x, p.y, button)
  await sleep(SETTLE)
}

async function key(keyName, modifiers = 0) {
  const code = keyName.length === 1 ? `Key${keyName.toUpperCase()}` : keyName
  await call('Input.dispatchKeyEvent', { type: 'keyDown', key: keyName, code, modifiers, windowsVirtualKeyCode: keyName.length === 1 ? keyName.toUpperCase().charCodeAt(0) : { Escape: 27, Enter: 13, Delete: 46 }[keyName] })
  await call('Input.dispatchKeyEvent', { type: 'keyUp', key: keyName, code, modifiers })
  await sleep(SETTLE)
}

async function shot(name) {
  const { data } = await call('Page.captureScreenshot', { format: 'png' })
  fs.writeFileSync(path.join(shots, `${name}.png`), Buffer.from(data, 'base64'))
}

// the page may not have a body yet right after the app starts
const text = async () => (await evaluate('document.body?.innerText')) ?? ''
const rows = () => evaluate("[...document.querySelectorAll('[role=row]')].map(r => r.innerText).join(' | ')")

// ---------- tests ----------
const results = []
async function test(name, body) {
  // --only <words>: just the tests whose name has them
  if (args.includes('--only') && !name.includes(args[args.indexOf('--only') + 1])) return
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
async function until(check, message, ms = 5000) {
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

/** Texts that don't fit: cut by their own box or by a box around them, sticking out of the window, or a placeholder
 *  longer than its input. Account rows are left out (long names are cut on purpose). */
const CUT_OFF = `(() => {
  const out = new Set()
  const measure = document.createElement('canvas').getContext('2d')
  const shown = (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden' }
  const name = (e) => (e.innerText || '').trim().split('\\n')[0].slice(0, 70)
  for (const e of document.querySelectorAll('body *')) {
    if (!shown(e) || e.closest('[role=row], textarea, pre, [data-sonner-toaster], .sr-only')) continue
    if (![...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim())) continue
    const style = getComputedStyle(e)
    if (style.display === 'inline' || /auto|scroll/.test(style.overflowX)) continue
    const r = e.getBoundingClientRect()
    if (e.scrollWidth > e.clientWidth + 1 || r.right > innerWidth + 1) {
      out.add(name(e))
      continue
    }
    for (let a = e.parentElement; a && a !== document.body; a = a.parentElement) {
      if (getComputedStyle(a).overflowX === 'visible') continue
      const box = a.getBoundingClientRect()
      if (r.right > box.right + 1 || r.left < box.left - 1) out.add(name(e))
      break
    }
  }
  for (const i of document.querySelectorAll('input')) {
    if (!shown(i) || i.value || !i.placeholder) continue
    const style = getComputedStyle(i)
    measure.font = style.font
    const room = i.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight)
    if (measure.measureText(i.placeholder).width > room + 1) out.add('placeholder: ' + i.placeholder)
  }
  return [...out]
})()`

/** Buttons, boxes and text areas of the main window drawn over each other */
const OVERLAPS = `(() => {
  const shown = (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 }
  const items = [...document.querySelectorAll('header button, header input, main button, main input, main textarea, aside button, aside input, aside textarea')]
    .filter((e) => shown(e) && !e.closest('[role=row]'))
  const out = []
  for (let i = 0; i < items.length; i++) {
    for (let j = i + 1; j < items.length; j++) {
      const [a, b] = [items[i], items[j]]
      if (a.contains(b) || b.contains(a)) continue
      const [r, q] = [a.getBoundingClientRect(), b.getBoundingClientRect()]
      if (Math.min(r.right, q.right) - Math.max(r.left, q.left) > 1 && Math.min(r.bottom, q.bottom) - Math.max(r.top, q.top) > 1) {
        const name = (e) => (e.innerText || e.placeholder || e.getAttribute('aria-label') || e.tagName).trim().split('\\n')[0]
        out.push('overlap: ' + name(a) + ' / ' + name(b))
      }
    }
  }
  return out
})()`

/** Clicks every tab of the open window in turn, checking each one */
async function eachTab(check) {
  const count = await evaluate(`document.querySelectorAll('[role=tab]').length`)
  for (let i = 0; i < count; i++) {
    const p = await evaluate(`(() => { const r = document.querySelectorAll('[role=tab]')[${i}].getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`)
    await mouse('mouseMoved', p.x, p.y)
    await mouse('mousePressed', p.x, p.y)
    await mouse('mouseReleased', p.x, p.y)
    await sleep(SETTLE)
    await check(await evaluate(`document.querySelectorAll('[role=tab]')[${i}].innerText.trim()`))
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function main() {
  prepare()
  const app = spawn(path.join(work, 'RobloxAccountManager.exe'), [], {
    cwd: work,
    // the test copy's window is off screen: Chromium must keep drawing it anyway
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --disable-features=CalculateNativeWinOcclusion --disable-backgrounding-occluded-windows --disable-renderer-backgrounding` },
    stdio: 'ignore',
  })

  try {
    await connect()
    await call('Page.enable')
    await call('Runtime.enable')
    // the test copy never has the Windows focus (it must not take it from you): the page behaves as if it had
    await call('Emulation.setFocusEmulationEnabled', { enabled: true })
    await until(async () => (await text()).includes('6 accounts'), 'the accounts never showed up', 15000)

    await test('main window lists the accounts by group', async () => {
      const t = await text()
      for (const name of ['MainAccount', 'StorageAlt01', 'GrindBot_B', 'MAIN', 'STORAGE', 'FARM', '6 accounts']) assert(t.includes(name), `missing ${name}`)
      // no game chosen: the README pictures don't show any game
      assert(t.includes('No game chosen'), 'a game is preset')
      await shot('main')
    })

    await test('a Status column says what each account is doing', async () => {
      assert((await text()).includes('Status'), 'no Status column')
      const statuses = await evaluate(`[...document.querySelectorAll('[role=row] [data-testid=status]')].map((s) => s.innerText.trim())`)
      const count = await evaluate(`document.querySelectorAll('[role=row]').length`)
      // made-up accounts: Roblox knows nothing about them
      assert(statuses.length === count && statuses.every((s) => s === 'Offline'), `statuses: ${statuses.join(', ')}`)
    })

    await test('the typed Place ID is remembered for the next start', async () => {
      const saved = () => /SavedPlaceId=(.*)/.exec(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8'))?.[1]?.trim()
      const type = (value) => evaluate(`(() => { const i = document.querySelector('#place'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(i, '${value}'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await type('1818')
      await until(async () => saved() === '1818', `not saved: ${saved()}`)
      await type('')
      await until(async () => (saved() ?? '') === '', `not emptied: ${saved()}`)
      // the server box too, by itself (there's no save button)
      const savedJob = () => /SavedJobId=(.*)/.exec(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8'))?.[1]?.trim()
      const typeJob = (value) => evaluate(`(() => { const i = document.querySelector('#job'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(i, '${value}'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await typeJob('https://www.roblox.com/share?code=abc123&type=Server')
      await until(async () => savedJob() === 'https://www.roblox.com/share?code=abc123&type=Server', `server box not saved: ${savedJob()}`)
      assert((await text()).includes('Private server from a link'), 'the panel does not say the link decides the game')
      await typeJob('')
      await until(async () => (savedJob() ?? '') === '', `server box not emptied: ${savedJob()}`)
    })
    await test('selecting enables the account buttons', async () => {
      await checkCursors('button, [role=row]')
      await click('MainAccount', '[role=row]')
      await checkCursors('button, [role=row]')
      const disabled = await evaluate(`[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Remove').disabled`)
      assert(!disabled, 'Remove still disabled')
      assert((await evaluate(`document.querySelector('input[placeholder="Alias"]').value`)) === 'Main', 'alias box')
    })

    await test('alias and description save by themselves, even when another account is chosen right away', async () => {
      const account = (name) => JSON.parse(fs.readFileSync(path.join(work, 'AccountData.json'), 'utf8')).find((a) => a.Username === name)
      const type = (selector, value) => evaluate(`(() => {
        const e = document.querySelector(${JSON.stringify(selector)})
        const proto = e.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype
        Object.getOwnPropertyDescriptor(proto, 'value').set.call(e, ${JSON.stringify(value)})
        e.dispatchEvent(new Event('input', { bubbles: true }))
      })()`)
      assert(!(await find('Set Alias', 'button')) && !(await find('Set Description', 'button')), 'save buttons still there')
      await click('BuilderAccount', '[role=row]')
      await type('input[placeholder="Alias"]', 'Builder 2')
      await until(async () => account('BuilderAccount')?.Alias === 'Builder 2', `alias not saved: ${account('BuilderAccount')?.Alias}`)
      await until(() => evaluate(`!!document.querySelector('[data-testid=saved]')`), 'no "Saved" sign')
      // typed, then straight to another account: it goes to the account it was typed for
      await type('textarea[placeholder="Description"]', 'builds things')
      await click('MainAccount', '[role=row]')
      await until(async () => account('BuilderAccount')?.Description === 'builds things', `description not saved: ${account('BuilderAccount')?.Description}`)
      assert(account('MainAccount')?.Description === 'Main account', 'saved into the wrong account')
      assert((await evaluate(`document.querySelector('input[placeholder="Alias"]').value`)) === 'Main', 'the alias box did not follow the selection')
      await click('BuilderAccount', '[role=row]')
      await type('input[placeholder="Alias"]', 'Builder')
      await type('textarea[placeholder="Description"]', '')
      await until(async () => account('BuilderAccount')?.Alias === 'Builder' && account('BuilderAccount')?.Description === '', 'not put back')
      await click('MainAccount', '[role=row]')
    })

    await test('each account keeps its own game, or the same for all', async () => {
      const savedOf = (name) => JSON.parse(fs.readFileSync(path.join(work, 'AccountData.json'), 'utf8')).find((a) => a.Username === name)?.Fields?.SavedPlaceId
      const placeBox = () => evaluate(`document.querySelector('#place').value`)
      const type = (value) => evaluate(`(() => { const i = document.querySelector('#place'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(i, '${value}'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      // MainAccount is selected: what's typed is saved into it
      await type('1818')
      await until(async () => savedOf('MainAccount') === '1818', `not saved into the account: ${savedOf('MainAccount')}`)
      // an account without its own game shows the last one used, and choosing back shows its own
      await click('StorageAlt01', '[role=row]')
      await until(async () => (await placeBox()) === '1818', `StorageAlt01 shows ${await placeBox()}`)
      assert(savedOf('StorageAlt01') === undefined, 'saved into an account just by choosing it')
      await type('920587237')
      await until(async () => savedOf('StorageAlt01') === '920587237', 'not saved into StorageAlt01')
      await click('MainAccount', '[role=row]')
      await until(async () => (await placeBox()) === '1818', `MainAccount shows ${await placeBox()}`)
      // same for all: choosing another account doesn't change the game
      await click('Same for all', 'label')
      await until(async () => /SameGameForAll=true/.test(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')), 'switch not saved')
      await click('StorageAlt01', '[role=row]')
      await sleep(300)
      assert((await placeBox()) === '1818', `the game changed with "same for all": ${await placeBox()}`)
      await click('Same for all', 'label')
      await until(async () => /SameGameForAll=false/.test(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')), 'switch not turned off')
      // empty again, so no picture after this shows a game
      await type('')
      await until(async () => savedOf('StorageAlt01') === undefined, 'StorageAlt01 kept its game')
      await click('MainAccount', '[role=row]')
      await type('')
      await until(async () => savedOf('MainAccount') === undefined && (await text()).includes('No game chosen'), 'MainAccount kept its game')
    })

    await test('the divider resizes the panels and remembers it', async () => {
      const sidebarWidth = () => evaluate(`Math.round(document.querySelector('[role=separator]').nextElementSibling.getBoundingClientRect().width)`)
      const saved = () => /SidebarWidth=(\d+)/.exec(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8'))?.[1]
      const before = await sidebarWidth()
      const p = await evaluate(`(() => { const r = document.querySelector('[role=separator]').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 } })()`)
      await mouse('mouseMoved', p.x, p.y)
      await mouse('mousePressed', p.x, p.y)
      for (let dx = 20; dx <= 120; dx += 20) await mouse('mouseMoved', p.x - dx, p.y)
      await mouse('mouseReleased', p.x - 120, p.y)
      await until(async () => (await sidebarWidth()) >= before + 110, `the panel didn't grow: ${before} -> ${await sidebarWidth()}`)
      await until(async () => Number(saved()) === (await sidebarWidth()), `width not saved: ${saved()}`)
      // double click: back to the default
      await evaluate(`document.querySelector('[role=separator]').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))`)
      await until(async () => (await sidebarWidth()) === 360 && saved() === '360', `not back to 360: ${await sidebarWidth()}`)
    })

    await test('search filters and counts', async () => {
      await evaluate(`(() => { const i = document.querySelector('input[placeholder="Search accounts..."]'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, 'storage'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await until(async () => (await text()).includes('2 of 6 accounts'), 'counter')
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
      await openSub('Color')
      await checkCursors('[role=menuitem]')
      await shot('color-menu')
      await click('Red', '[role=menuitem]')
      await until(async () => (await style(name))?.startsWith('rgb(239, 68, 68)|rgb(239, 68, 68)'), 'the account is not red')

      // its group in blue: the header, and the bar of the group's other account
      await click('FARM', 'button', 'right')
      await openSub('Group color')
      await click('Blue', '[role=menuitem]')
      await until(async () => (await style(header))?.startsWith('rgb(59, 130, 246)'), 'the group is not blue')
      const other = `[...document.querySelectorAll('[role=row] span.font-medium')].find((e) => e.innerText === 'GrindBot_B')`
      await until(async () => (await style(other))?.endsWith('rgb(59, 130, 246)'), "the group's color is not on its accounts")
      // the account's own color wins over its group's
      assert((await style(name)).endsWith('rgb(239, 68, 68)'), "the account's color lost to the group's")
      await shot('colors')

      // and back to no colors
      await click('GrindBot_A', '[role=row]', 'right')
      await openSub('Color')
      await click('No color', '[role=menuitem]')
      await click('FARM', 'button', 'right')
      await openSub('Group color')
      await click('No color', '[role=menuitem]')
      await until(async () => (await style(other))?.endsWith('|'), 'the colors did not go away')
    })

    await test('CPU limit for an account, from the right click menu', async () => {
      const badge = () => evaluate(`[...document.querySelectorAll('[role=row]')].find((r) => r.innerText.includes('BuilderAccount'))?.innerText.includes('CPU 10%')`)
      await click('BuilderAccount', '[role=row]', 'right')
      await openSub('CPU limit')
      await shot('cpu-menu')
      await click('10%', '[role=menuitem]')
      await until(badge, 'no "CPU 10%" on the account')
      await click('BuilderAccount', '[role=row]', 'right')
      await openSub('CPU limit')
      await click('No limit', '[role=menuitem]')
      await until(async () => !(await badge()), 'the limit did not go away')
    })
    await test('rename a group from its header', async () => {
      await menuItem('FARM', 'button', 'Rename group…')
      await evaluate(`(() => { const i = document.querySelector('[role=dialog] input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '3Grinders'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await evaluate(`document.querySelector('[role=dialog] form').requestSubmit()`)
      await until(async () => (await text()).includes('GRINDERS') && !(await text()).includes('FARM'), 'the group was not renamed')
      // and back, so the other tests find it
      await menuItem('GRINDERS', 'button', 'Rename group…')
      await evaluate(`(() => { const i = document.querySelector('[role=dialog] input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '3Farm'); i.dispatchEvent(new Event('input', { bubbles: true })) })()`)
      await evaluate(`document.querySelector('[role=dialog] form').requestSubmit()`)
      await until(async () => (await text()).includes('FARM'), 'the group did not get its name back')
    })

    await test('remove shows undo and undo brings it back', async () => {
      await click('StorageAlt02', '[role=row]')
      // the click must have selected it (a busy PC can miss it), Remove takes the selection
      const selectedNow = () => evaluate(`[...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.innerText.split('\\n')[0]).join(',')`)
      await until(async () => (await selectedNow()) === 'StorageAlt02', `selection: ${await selectedNow()}`).catch(async () => {
        await click('StorageAlt02', '[role=row]')
        await until(async () => (await selectedNow()) === 'StorageAlt02', `StorageAlt02 isn't selected, the selection is "${await selectedNow()}"`)
      })
      await click('Remove', 'button')
      const toasts = () => evaluate(`[...document.querySelectorAll('[data-sonner-toast]')].map((t) => t.innerText).join(' | ')`)
      await until(async () => !(await rows()).includes('StorageAlt02'), 'not removed').catch(async (e) => { throw new Error(`${e.message}; selection "${await selectedNow()}", notifications: ${await toasts()}`) })
      // the notification slides in: a click while it moves can miss the button
      await sleep(500)
      await click('Undo', 'button')
      await until(async () => (await rows()).includes('StorageAlt02'), 'undo failed').catch(async () => {
        await click('Undo', 'button')
        await until(async () => (await rows()).includes('StorageAlt02'), 'undo failed, twice')
      })
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

    await test('turning "Save Passwords" off offers to forget the saved ones', async () => {
      const passwordOf = (name) => JSON.parse(fs.readFileSync(path.join(work, 'AccountData.json'), 'utf8')).find((a) => a.Username === name)?.Password
      const ini = () => fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')
      // the switch whose row (its first parent with any text) is the Save Passwords one
      const toggle = () => evaluate(`[...document.querySelectorAll('[data-page] [role=switch]')].find((s) => {
        let row = s.parentElement
        while (row && !row.innerText.trim()) row = row.parentElement
        return row?.innerText.includes('Save Passwords')
      })?.click()`)
      // off by default
      assert(/SavePasswords=false/.test(ini()), 'passwords are saved by default')
      await click('Settings', 'button')
      await toggle()
      await until(async () => /SavePasswords=true/.test(ini()), 'not turned on')
      await toggle()
      await until(async () => (await text()).includes('Remove saved passwords?'), 'no offer to remove the saved passwords')
      await click('Yes', 'button')
      await until(async () => passwordOf('GrindBot_B') === '', `the saved password is still there: ${passwordOf('GrindBot_B')}`)
      assert(/SavePasswords=false/.test(ini()), 'not turned off')
      await key('Escape')
    })

    await test('automatic updates can be turned off with one choice', async () => {
      const ini = () => fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')
      const updates = () => /\[Updates\][^[]*/.exec(ini())?.[0] ?? ''
      await click('Settings', 'button')
      // to the middle: a notification in the bottom right corner could be over it
      await evaluate(`[...document.querySelectorAll('[data-slot=setting-row], div')].find((e) => e.innerText?.startsWith('Automatic updates'))?.scrollIntoView({ block: 'center' })`)
      // the default: told about a new version, nothing installed without a click
      assert(/Check=true/.test(updates()) && /AutoInstall=false/.test(updates()), `the default isn't "Only tell me":\n${updates()}`)
      await click('Only tell me', '[role=combobox]')
      await click('Off', '[role=option]')
      await until(async () => /Check=false/.test(updates()) && /AutoInstall=false/.test(updates()), `not saved as off:\n${updates()}`)
      assert((await text()).includes('never contacts GitHub'), 'no explanation for "Off"')
      await click('Off', '[role=combobox]')
      await click('Automatic', '[role=option]')
      await until(async () => /Check=true/.test(updates()) && /AutoInstall=true/.test(updates()), `not saved as automatic:\n${updates()}`)
      await click('Automatic', '[role=combobox]')
      await click('Only tell me', '[role=option]')
      await until(async () => /Check=true/.test(updates()) && /AutoInstall=false/.test(updates()), `not back to "Only tell me":\n${updates()}`)
      await key('Escape')
    })

    await test('theme editor, in Settings > Appearance', async () => {
      await click('Settings', 'button')
      await click('Appearance', '[role=tab]')
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

    await test('anti-afk interval: fixed minutes or random between two', async () => {
      const afk = () => /\[AntiAFK\][^[]*/.exec(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8'))?.[0] ?? ''
      // types into the n-th minutes box of the Behaviour tab and leaves it (that saves)
      const minutes = (n, value) => evaluate(`(() => { const i = document.querySelectorAll('[data-page] input[type=number][step="0.5"]')[${n}]; i.focus(); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(i, '${value}'); i.dispatchEvent(new Event('input', { bubbles: true })); i.blur() })()`)
      await click('Anti-AFK', 'button')
      await click('Behaviour', '[role=tab]')
      await minutes(0, '7.5')
      await until(async () => /\nInterval=450\r?\n/.test(afk()), `fixed 7.5 minutes not saved:\n${afk()}`)
      await click('Fixed', '[role=combobox]')
      await click('Random', '[role=option]')
      await until(async () => (await text()).includes('Between (minutes)'), 'no random limits')
      await minutes(0, '4')
      await minutes(1, '25')
      // the high limit can't go past 19 minutes
      await until(async () => /IntervalMode=random/.test(afk()) && /IntervalMin=240/.test(afk()) && /IntervalMax=1140/.test(afk()), `random limits not saved:\n${afk()}`)
      await shot('antiafk-random-interval')
      await click('Random', '[role=combobox]')
      await click('Fixed', '[role=option]')
      await until(async () => /IntervalMode=fixed/.test(afk()), 'not back to fixed')
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
        // (with the README's normal settings the round runs instead of waiting, for a made-up account: no keys)
        if (!args.includes('--readme')) await until(async () => (await text()).includes('Waiting for you to stop typing and moving the mouse'), 'no waiting state', 8000)
      } else {
        await until(async () => (await text()).includes('Waiting for a Roblox window'), 'no "waiting for a Roblox window" state')
      }

      // the session that's running already counts as the longest one (a session ended by the app closing used to be lost)
      const longest = () => evaluate(`window.__TAURI_INTERNALS__.invoke('antiafk_status').then((s) => s.stats.longest)`)
      await sleep(2500)
      const seconds = await longest()
      assert(seconds >= 2, `the running session doesn't count as the longest: ${seconds}`)

      await click('Stop', 'button')
      await until(async () => (await text()).includes('Stopped') && (await find('Start', 'button')) !== null, 'Stop did not change to Stopped / Start')
      await key('Escape')
    })

    await test('resetting the statistics keeps them in a history, where each one can be deleted', async () => {
      const ini = () => fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8')
      const past = () => evaluate(`document.querySelectorAll('[data-testid=past-stats]').length`)
      await click('Anti-AFK', 'button')
      await click('Statistics', '[role=tab]')
      assert((await past()) === 0, 'a history before any reset')
      await click('Reset statistics', 'button')
      await click('Yes', 'button')
      await until(async () => (await past()) === 1, 'the reset statistics are not in the history')
      assert(/StatHistory=\[\{.*"actions":30/.test(ini()) && /StatActions=0/.test(ini()), `not kept or not reset: ${ini().split('\n').filter((l) => l.startsWith('Stat')).join(' ')}`)
      assert((await text()).includes('Oct 9, 2025'), 'the dates they cover are not shown')
      await shot('antiafk-history')
      // nothing counted since: a second reset adds nothing
      await click('Reset statistics', 'button')
      await click('Yes', 'button')
      await sleep(300)
      assert((await past()) === 1, 'an empty reset was kept')
      await click('Delete', 'button')
      await click('Yes', 'button')
      await until(async () => (await past()) === 0, 'not deleted')
      assert(/StatHistory=\[\]/.test(ini()), 'still in RAMSettings.ini')
      await key('Escape')
    })

    await test('the pages of the navigation rail, and Esc back to the accounts', async () => {
      const page = () => evaluate(`document.querySelector('[data-page]')?.getAttribute('data-page') ?? 'accounts'`)
      await click('Games', 'button')
      assert((await page()) === 'games', 'no Games page')
      for (const tab of ['Servers', 'Favorites', 'Universe']) {
        await click(tab, '[role=tab]')
        await shot(`utilities-${tab.toLowerCase()}`)
      }
      await click('Avatar', 'button')
      assert((await page()) === 'avatar', 'no Avatar page')
      await shot('utilities-outfits')
      await click('Settings', 'button')
      await click('Watcher', '[role=tab]')
      await shot('utilities-watcher')
      await key('Escape')
      await until(async () => (await page()) === 'accounts', 'Esc did not go back to the accounts')
      assert((await text()).includes('MainAccount'), 'the account list is not back')
    })

    await test('Instances lists the Roblox windows, with their tools', async () => {
      // only looks: the Roblox windows of this PC (yours too) are never clicked
      await click('Instances', 'button')
      const body = await text()
      assert(body.includes('Every Roblox open on this PC'), 'no Instances page')
      assert(await find('Arrange in a grid', 'button'), 'no window tools')
      const shown = await evaluate(`document.querySelectorAll('[data-testid=instance]').length`)
      const windows = await evaluate(`window.__TAURI_INTERNALS__.invoke('antiafk_status').then((s) => s.windows.length)`)
      assert(shown === windows, `${shown} rows for ${windows} Roblox windows`)
      if (!windows) assert(body.includes('No Roblox windows open'), 'no empty state')
      await key('Escape')
    })

    await test('Friends: an account, its friends and requests (made-up accounts: Roblox refuses them, the page says so)', async () => {
      await click('Friends', 'button')
      assert((await text()).includes("An account's friends"), 'no Friends page')
      // the made-up cookies get nothing from Roblox: the page ends empty, not stuck loading
      await until(async () => !(await text()).includes('Loading…'), 'still loading', 20000)
      assert((await text()).includes('No friends yet'), 'no empty state')
      assert(await find('In game', 'button') && await find('Offline', 'button'), 'no status filters')
      await click('Requests', '[role=tab]')
      await until(async () => (await text()).includes('No friend requests'), 'no empty requests state')
      await key('Escape')
    })

    await test('History shows the events by day, filters by account and clears', async () => {
      const events = () => evaluate(`[...document.querySelectorAll('[data-testid=history-event]')].map((e) => e.innerText.replace(/\\s+/g, ' ')).join(' | ')`)
      await click('History', 'button')
      await until(async () => (await events()).includes('Lost the connection'), `events not shown: ${await events()}`)
      const all = await events()
      assert(all.includes('Rejoined') && all.includes('Joined a game') && all.includes('Demo Game'), `missing events: ${all}`)
      // newest first: MainAccount going offline (an hour ago) comes before its joining yesterday
      assert(all.indexOf('Offline') < all.indexOf('Joined a game'), `not newest first: ${all}`)
      await shot('history')
      await click('All accounts', '[role=combobox]')
      await click('GrindBot_A', '[role=option]')
      await until(async () => !(await events()).includes('MainAccount'), 'the account filter does nothing')
      await click('Clear', 'button')
      await click('Yes', 'button')
      await until(async () => (await evaluate(`document.querySelectorAll('[data-testid=history-event]').length`)) === 0, 'not cleared')
      assert(!fs.existsSync(path.join(work, 'SessionHistory.json')), 'the history file is still there')
      await key('Escape')
    })

    await test('Roblox home, window names and capture privacy are there', async () => {
      await click('MainAccount', '[role=row]')
      // the other ways to join are in the Join Server button's menu
      await click('More ways to join', 'button')
      assert(await find('Open Roblox home', '[role=menuitem]') && await find('Follow a player', '[role=menuitem]'), 'the Join menu misses Roblox home or Follow')
      await key('Escape')
      await click('Settings', 'button')
      const body = await text()
      assert(body.includes('Name Roblox windows after their account') && body.includes('Hide this app from screen captures'), 'settings missing')
      await key('Escape')
    })

    await test('a notification closes with its X while a window is open', async () => {
      await click('MainAccount', '[role=row]')
      await click('Account Utilities', 'button')
      // the same event Rust sends for an error
      await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'toast', payload: { kind: 'error', message: 'Test notification' } })`)
      await until(async () => !!(await evaluate(`!!document.querySelector('[data-close-button]')`)), 'no notification')
      await sleep(400)
      const p = await evaluate(`(() => { const r = document.querySelector('[data-close-button]').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 } })()`)
      for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await mouse(type, p.x, p.y)
      await until(async () => !(await text()).includes('Test notification'), 'the X did not close the notification')
      assert((await text()).includes('Change Password'), 'the window closed instead')
      await key('Escape')
    })

    await test('account utilities', async () => {
      await click('MainAccount', '[role=row]')
      await click('Account Utilities', 'button')
      assert((await text()).includes('Change Password'), 'not open')
      await shot('account-utilities')
      await key('Escape')
    })

    await test('nothing is cut off or drawn over something else, in any language, at the default and the smallest window size', async () => {
      const cut = []
      // each text once, where it was first seen (the main window stays behind the open ones)
      const seen = new Set()
      const look = async (where) => {
        for (const c of await evaluate(CUT_OFF)) {
          if (seen.has(c)) continue
          seen.add(c)
          cut.push(`${where}: ${c}`)
        }
      }
      const languages = [['en', 'EN', 'English'], ['pt-PT', 'PT', 'Português (Portugal)'], ['es', 'ES', 'Español'], ['fr', 'FR', 'Français'], ['de', 'DE', 'Deutsch']]
      let current = 'EN'
      try {
        for (const [code, flag, name] of languages) {
          await click(current, 'button')
          await click(name, '[role=menuitem]')
          current = flag
          const table = code === 'en' ? {} : JSON.parse(fs.readFileSync(path.join(root, 'lang', `${code}.json`), 'utf8'))
          const tr = (s) => table[s] ?? s
          await until(async () => (await text()).includes(tr('Add Account')), `${name} not shown`)
          await click('MainAccount', '[role=row]')
          await look(`${code} main`)
          for (const o of await evaluate(OVERLAPS)) cut.push(`${code} main: ${o}`)
          await call('Emulation.setDeviceMetricsOverride', { width: 900, height: 560, deviceScaleFactor: 1, mobile: false })
          await sleep(SETTLE)
          await look(`${code} main at 900px`)
          for (const o of await evaluate(OVERLAPS)) cut.push(`${code} main at 900px: ${o}`)
          if (code === 'de') await shot('main-de-900')
          await call('Emulation.clearDeviceMetricsOverride')
          await sleep(SETTLE)
          for (const [button, selector] of [[tr('Settings'), 'button'], ['Anti-AFK', 'button'], [tr('Instances'), 'button'], [tr('Friends'), 'button'], [tr('Games'), 'button'], [tr('Avatar'), 'button'], [tr('History'), 'button'], [tr('Account Utilities'), 'button']]) {
            await click(button, selector)
            const tabs = await evaluate(`document.querySelectorAll('[role=tab]').length`)
            if (tabs) await eachTab((tab) => look(`${code} ${button} > ${tab}`))
            else await look(`${code} ${button}`)
            await key('Escape')
          }
        }
      } finally {
        await call('Emulation.clearDeviceMetricsOverride')
        if (current !== 'EN') {
          await click(current, 'button')
          await click('English', '[role=menuitem]')
        }
      }
      assert(cut.length === 0, `cut off:\n        ${cut.join('\n        ')}`)
    })

    await test('language switch is instant', async () => {
      await click('EN', 'button')
      await click('Português (Portugal)', '[role=menuitem]')
      await until(async () => (await text()).includes('Adicionar conta'), 'not translated')
      await shot('main-pt')
      for (const [code, name, word] of [['PT', 'Español', 'Añadir cuenta'], ['ES', 'Français', 'Ajouter un compte'], ['FR', 'Deutsch', 'Konto hinzufügen']]) {
        await click(code, 'button')
        await click(name, '[role=menuitem]')
        await until(async () => (await text()).includes(word), `${name} not translated`)
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
    colors: 'colors', 'cpu-menu': 'cpu-limit',
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
