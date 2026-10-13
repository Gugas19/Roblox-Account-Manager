// Command line tests: node tests/cli.mjs [--release]
//
// Runs ram-cli in target/test-run/cli with demo accounts, first on its own, then through an open app.
// Nothing here launches Roblox, closes a process or presses keys: join, close and antiafk are only
// tried on the paths that stop before doing anything (wrong names, missing options, app not open).

import { spawn, spawnSync, execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const build = process.argv.includes('--release') ? 'release' : 'debug'
const target = path.join(root, 'target', build)
const work = path.join(target, '..', 'test-run', 'cli')
const locked = path.join(target, '..', 'test-run', 'cli-password')

function stopTestApps() {
  try { execSync(`powershell -NoProfile -Command "Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '*\\target\\test-run\\cli*' } | Stop-Process -Force"`) } catch {}
}

// Anti-AFK in the test copy can never press a key: it only takes a made-up account, and waits for a day without
// input first. The real Roblox on this PC (if open) is listed but never touched.
const SAFE_ANTI_AFK = '[AntiAFK]\r\nAccounts=__test_nobody__\r\nUserSafe=true\r\nIdleSeconds=86400\r\nMaxWaitSeconds=86400\r\nKeepAwake=false\r\nAutoStart=false\r\nAutoStartIdle=false\r\nAutoRejoin=false\r\nMuteBackground=false\r\nEfficiency=false\r\nAutoGrid=false\r\n'

function prepare(dir) {
  fs.rmSync(dir, { recursive: true, force: true })
  fs.mkdirSync(dir, { recursive: true })
  for (const exe of ['RobloxAccountManager.exe', 'ram-cli.exe']) fs.copyFileSync(path.join(target, exe), path.join(dir, exe))
  // a test copy: its own lock and browser data, so it runs next to your own app without touching it
  fs.writeFileSync(path.join(dir, 'ram-test-copy'), '')
  fs.writeFileSync(path.join(dir, 'RAMSettings.ini'), '[General]\r\nLanguage=en\r\nEnableMultiRbx=false\r\n' + SAFE_ANTI_AFK)
}

stopTestApps()

prepare(work)
fs.writeFileSync(path.join(work, 'NoEncryption.IUnderstandTheRisks.iautamor'), '')
const demo = (Username, UserID, Group, Alias, Description = '') => ({ Username, UserID, Group, Alias, LastUse: new Date().toISOString(), SecurityToken: `fake-token-${UserID}`, Description, Password: '', Fields: {} })
fs.writeFileSync(path.join(work, 'AccountData.json'), JSON.stringify([
  demo('NightFalcon_07', 1, '1Main', 'Main', 'Main account'), demo('PixelBuilderBR', 2, '1Main', 'Builder'),
  demo('StorageAlt01', 3, '2Storage', 'Bank'), demo('GrindBot_A', 5, '3Farm', 'Farm A'), demo('GrindBot_B', 6, '3Farm', 'Farm B'),
]))
prepare(locked)
fs.copyFileSync(path.join(root, 'tests', 'fixtures', 'password-teste1234.bin'), path.join(locked, 'AccountData.json'))

function cli(dir, args, env = {}) {
  const r = spawnSync(path.join(dir, 'ram-cli.exe'), args, { encoding: 'utf8', env: { ...process.env, RAM_PASSWORD: '', ...env }, timeout: 60000, input: '' })
  return { code: r.status, out: r.stdout, err: r.stderr }
}

let passed = 0, failed = 0
async function test(name, body) {
  try {
    await body()
    passed++
    console.log(`  ok    ${name}`)
  } catch (e) {
    failed++
    console.log(`  FAIL  ${name}\n        ${e.message.replace(/\n/g, '\n        ')}`)
  }
}
function expect(r, code, ...texts) {
  const all = r.out + r.err
  if (r.code !== code) throw new Error(`exit code ${r.code}, expected ${code}\n${all}`)
  for (const t of texts) if (!all.includes(t)) throw new Error(`"${t}" missing from:\n${all}`)
}

console.log('On its own')
await test('help and version', () => {
  expect(cli(work, ['help']), 0, 'Usage: ram-cli', 'follow <accounts>')
  expect(cli(work, ['--version']), 0, 'ram-cli 4.')
})
await test('list shows a table without cookies', () => {
  const r = cli(work, ['list'])
  expect(r, 0, 'USERNAME', 'NightFalcon_07', 'Main account', 'Farm')
  if (r.out.includes('fake-token')) throw new Error('a cookie was printed')
})
await test('list --json and --group', () => {
  const all = JSON.parse(cli(work, ['list', '--json']).out)
  if (all.length !== 5 || JSON.stringify(all).includes('fake-token')) throw new Error(JSON.stringify(all))
  const farm = JSON.parse(cli(work, ['list', '--group', 'farm', '--json']).out)
  if (farm.map((a) => a.username).join() !== 'GrindBot_A,GrindBot_B') throw new Error(JSON.stringify(farm))
})
await test('status', () => expect(cli(work, ['status']), 0, 'not open', 'Accounts: 5', 'Anti-AFK: off'))
await test('wrong command lines exit with 2', () => {
  expect(cli(work, ['dance']), 2, "Unknown command 'dance'", 'ram-cli help')
  expect(cli(work, ['join']), 2, 'Name the accounts')
  expect(cli(work, ['join', 'Main', '--place', 'abc']), 2, '--place takes a place id')
  expect(cli(work, ['join', 'Main', '--pace', '1']), 2, 'Unknown option --pace')
  expect(cli(work, ['follow', 'Main']), 2, '--user')
  expect(cli(work, ['close']), 2, 'Name the accounts')
  expect(cli(work, ['import']), 2, 'No cookies')
})
await test('unknown accounts and groups fail before doing anything', () => {
  expect(cli(work, ['join', 'Nobody', '--place', '1818']), 1, 'No account named Nobody')
  expect(cli(work, ['close', '--group', 'Nope']), 1, 'No group named Nope')
})
await test('Anti-AFK needs the app open', () => {
  expect(cli(work, ['antiafk']), 0, 'Anti-AFK is off')
  expect(cli(work, ['antiafk', 'start']), 1, 'ram-cli open')
})
await test('password protected accounts', () => {
  expect(cli(locked, ['list']), 1, 'RAM_PASSWORD')
  const wrong = cli(locked, ['list'], { RAM_PASSWORD: 'wrong-password' })
  if (wrong.code !== 1) throw new Error(wrong.out + wrong.err)
  expect(cli(locked, ['list'], { RAM_PASSWORD: 'teste1234' }), 0, 'NightFalcon_07')
  // updating never needs the accounts, so it doesn't ask for their password (and test copies don't update)
  const update = cli(locked, ['update'])
  expect(update, 1, 'development build')
  if ((update.out + update.err).includes('RAM_PASSWORD')) throw new Error('update asked for the password')
})

console.log('Through the open app')
const app = spawn(path.join(work, 'RobloxAccountManager.exe'), [], { cwd: work, detached: true, stdio: 'ignore' })
try {
  let status = ''
  for (let i = 0; i < 60 && !status.includes('Accounts: 5'); i++) {
    await new Promise((r) => setTimeout(r, 500))
    status = cli(work, ['status']).out
  }
  await test('status comes from the app', () => { if (!status.includes(', open') || !status.includes('Accounts: 5')) throw new Error(status) })
  await test('list through the app', () => expect(cli(work, ['list', '--group', 'Main']), 0, 'NightFalcon_07', 'PixelBuilderBR'))
  await test('errors and exit codes through the app', () => {
    expect(cli(work, ['join', 'Nobody', '--place', '1818']), 1, 'No account named Nobody')
    expect(cli(work, ['join']), 2, 'Name the accounts')
  })
  await test('Anti-AFK status through the app', () => expect(cli(work, ['antiafk', 'status']), 0, 'Anti-AFK is off'))
  await test('Anti-AFK start and stop through the app, the status follows at once', () => {
    expect(cli(work, ['antiafk', 'start']), 0, 'Anti-AFK is on')
    expect(cli(work, ['antiafk', 'status']), 0, 'Anti-AFK is on')
    expect(cli(work, ['antiafk', 'stop']), 0, 'Anti-AFK is off')
  })
  await test('open shows the window', () => expect(cli(work, ['open']), 0, 'is open'))
  await test('several commands at once', async () => {
    const runs = await Promise.all(Array.from({ length: 5 }, () => new Promise((resolve) => {
      const p = spawn(path.join(work, 'ram-cli.exe'), ['list', '--json'], { env: { ...process.env, RAM_PASSWORD: '' } })
      let out = ''
      p.stdout.on('data', (d) => (out += d))
      p.on('close', (code) => resolve({ code, out }))
    })))
    for (const r of runs) if (r.code !== 0 || JSON.parse(r.out).length !== 5) throw new Error(JSON.stringify(r))
  })
} finally {
  app.kill()
  stopTestApps()
}

console.log(`\n${passed} passed, ${failed} failed`)
process.exit(failed ? 1 : 0)
