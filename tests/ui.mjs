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
const SAFE_ANTI_AFK = '[AntiAFK]\r\nAccounts=__test_nobody__\r\nUserSafe=true\r\nIdleSeconds=86400\r\nMaxWaitSeconds=86400\r\nKeepAwake=false\r\nAutoStart=false\r\nAutoStartIdle=false\r\nAutoRejoin=false\r\nAutoRejoinDefaultOn=true\r\nMuteBackground=false\r\nEfficiency=false\r\nAutoGrid=false\r\n'

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

/** Menus and dialogs animate for 100 ms: after an action, a short settle before the next one */const SETTLE = 150

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
      const disabled = await evaluate(`[...document.querySelectorAll('button')].find(b => b.innerText.includes('Remove')).disabled`)
      assert(!disabled, 'Remove still disabled')
      assert((await evaluate(`document.querySelector('input[placeholder="Alias"]').value`)) === 'Main', 'alias box')
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
      await click('Undo', 'button')
      await until(async () => (await rows()).includes('StorageAlt02'), 'undo failed')
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
      const toggle = () => evaluate(`[...document.querySelectorAll('[role=dialog] [role=switch]')].find((s) => s.closest('div.flex')?.innerText?.includes('Save Passwords'))?.click()`)
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
      await evaluate(`[...document.querySelectorAll('[data-slot=setting-row], div')].find((e) => e.innerText?.startsWith('Automatic updates'))?.scrollIntoView()`)
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

    await test('anti-afk interval: fixed minutes or random between two', async () => {
      const afk = () => /\[AntiAFK\][^[]*/.exec(fs.readFileSync(path.join(work, 'RAMSettings.ini'), 'utf8'))?.[0] ?? ''
      // types into the n-th minutes box of the Behaviour tab and leaves it (that saves)
      const minutes = (n, value) => evaluate(`(() => { const i = document.querySelectorAll('[role=dialog] input[type=number][step="0.5"]')[${n}]; i.focus(); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(i, '${value}'); i.dispatchEvent(new Event('input', { bubbles: true })); i.blur() })()`)
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

    await test('a notification closes with its X while a window is open', async () => {
      await click('Utilities', 'button')
      // the same event Rust sends for an error
      await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: 'toast', payload: { kind: 'error', message: 'Test notification' } })`)
      await until(async () => !!(await evaluate(`!!document.querySelector('[data-close-button]')`)), 'no notification')
      await sleep(400)
      const p = await evaluate(`(() => { const r = document.querySelector('[data-close-button]').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 } })()`)
      for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) await mouse(type, p.x, p.y)
      await until(async () => !(await text()).includes('Test notification'), 'the X did not close the notification')
      assert((await text()).includes('Servers'), 'the window closed instead')
      await key('Escape')
    })

    await test('account utilities', async () => {
      await click('MainAccount', '[role=row]')
      await click('Account Utilities', 'button')
      assert((await text()).includes('Change Password'), 'not open')
      await shot('account-utilities')
      await key('Escape')
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
