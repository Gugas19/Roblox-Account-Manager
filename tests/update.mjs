// Auto updater, end to end: node tests/update.mjs   (after cargo build --release)
//
// Runs the release build from a temporary folder (outside target, where updates are off) with a staged update:
// - signed with the release key: installed, and the new version starts
// - one byte changed in the zip: refused, the current version keeps running
// - a signed zip staged as a newer version than the one it was signed for: refused
// Needs the release signing key on this PC. Never touches GitHub, nor your own copy of the app.

import { execSync, spawn } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const release = path.join(root, 'target', 'release')
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const ps = (command) => execSync(`powershell -NoProfile -Command "${command}; exit 0"`, { encoding: 'utf8' }).trim()

const key = path.join(os.homedir(), '.roblox-account-manager', 'release-signing-key.txt')
if (!fs.existsSync(key)) {
  console.error('No release signing key on this PC, skipping')
  process.exit(0)
}

const version = /^version = "(\d+)\.(\d+)\.(\d+)"/m.exec(fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8')).slice(1).map(Number)
const next = `${version[0]}.${version[1]}.${version[2] + 1}`
const later = `${version[0]}.${version[1]}.${version[2] + 2}`
const base = fs.mkdtempSync(path.join(os.tmpdir(), 'ram-update-test-'))

// the "new version": the same exes in a signed zip
const zipDir = path.join(base, 'zip')
fs.mkdirSync(zipDir)
for (const exe of ['RobloxAccountManager.exe', 'ram-cli.exe']) fs.copyFileSync(path.join(release, exe), path.join(zipDir, exe))
const zip = path.join(base, `RobloxAccountManager-${next}.zip`)
ps(`Compress-Archive -Path '${zipDir}\\*' -DestinationPath '${zip}'`)
execSync(`cargo run -q --release --example sign-release -- "${zip}"`, { cwd: root, stdio: 'inherit' })

function install(name, { zipBytes = fs.readFileSync(zip), stagedVersion = next } = {}) {
  const dir = path.join(base, name)
  fs.mkdirSync(path.join(dir, 'update'), { recursive: true })
  for (const exe of ['RobloxAccountManager.exe', 'ram-cli.exe']) fs.copyFileSync(path.join(release, exe), path.join(dir, exe))
  fs.writeFileSync(path.join(dir, 'RAMSettings.ini'), '[General]\r\nLanguage=en\r\n[Updates]\r\nCheck=false\r\nAutoInstall=false\r\n[AntiAFK]\r\nAccounts=__test_nobody__\r\nIdleSeconds=86400\r\nKeepAwake=false\r\nBackground=false\r\n')
  fs.writeFileSync(path.join(dir, 'AccountData.json'), '[]')
  // a test copy: its own lock and browser data, so it runs next to your own app without touching it
  fs.writeFileSync(path.join(dir, 'ram-test-copy'), '')
  fs.writeFileSync(path.join(dir, 'NoEncryption.IUnderstandTheRisks.iautamor'), '')
  fs.writeFileSync(path.join(dir, 'update', 'update.zip'), zipBytes)
  fs.copyFileSync(`${zip}.sig`, path.join(dir, 'update', 'update.zip.sig'))
  fs.writeFileSync(path.join(dir, 'update', 'version.txt'), stagedVersion)
  return dir
}

const running = (dir) => ps(`Get-CimInstance Win32_Process -Filter \\"Name='RobloxAccountManager.exe'\\" | Where-Object { $_.ExecutablePath -like '${dir}\\*' } | ForEach-Object { $_.CommandLine }`)
const stop = (dir) => ps(`Get-Process RobloxAccountManager -ErrorAction SilentlyContinue | Where-Object { $_.Path -like '${dir}\\*' } | Stop-Process -Force`)
const log = (dir) => (fs.existsSync(path.join(dir, 'log.txt')) ? fs.readFileSync(path.join(dir, 'log.txt'), 'utf8') : '')

async function until(check, ms = 20000) {
  const end = Date.now() + ms
  while (Date.now() < end) {
    if (check()) return true
    await sleep(250)
  }
  return false
}

let failed = 0
async function test(name, body) {
  try {
    await body()
    console.log(`  ok    ${name}`)
  } catch (e) {
    failed++
    console.log(`  FAIL  ${name}\n        ${e.message}`)
  }
}
const assert = (condition, message) => { if (!condition) throw new Error(message) }

await test('a signed update is installed and the new version starts', async () => {
  const dir = install('good')
  spawn(path.join(dir, 'RobloxAccountManager.exe'), [], { cwd: dir, detached: true, stdio: 'ignore' }).unref()
  try {
    assert(await until(() => log(dir).includes(`Update ${next} installed`)), `not installed:\n${log(dir)}`)
    assert(await until(() => running(dir).includes('--updated')), 'the new version did not start')
    assert(await until(() => log(dir).includes('Started after an update')), `the new version did not get going:\n${log(dir)}`)
    assert(!fs.existsSync(path.join(dir, 'update')), 'update folder left behind')
    assert(fs.existsSync(path.join(dir, 'RobloxAccountManager.exe.old')), 'no backup of the previous version')
  } finally {
    stop(dir)
  }
})

await test('a changed zip is refused and the current version keeps running', async () => {
  const bytes = fs.readFileSync(zip)
  bytes[Math.floor(bytes.length / 2)] ^= 0xff
  const dir = install('tampered', { zipBytes: bytes })
  spawn(path.join(dir, 'RobloxAccountManager.exe'), [], { cwd: dir, detached: true, stdio: 'ignore' }).unref()
  try {
    assert(await until(() => log(dir).includes("isn't signed with the release key")), `not refused:\n${log(dir)}`)
    assert(await until(() => running(dir) !== ''), 'the app did not keep running')
    assert(!running(dir).includes('--updated'), 'an update was started')
    assert(!fs.existsSync(path.join(dir, 'update')), 'the refused update was kept')
    assert(!fs.existsSync(path.join(dir, 'RobloxAccountManager.exe.old')), 'files were swapped')
  } finally {
    stop(dir)
  }
})

await test('a signed zip passed off as a newer version is refused', async () => {
  const dir = install('relabelled', { stagedVersion: later })
  spawn(path.join(dir, 'RobloxAccountManager.exe'), [], { cwd: dir, detached: true, stdio: 'ignore' }).unref()
  try {
    assert(await until(() => log(dir).includes("isn't signed with the release key")), `not refused:\n${log(dir)}`)
    assert(!running(dir).includes('--updated'), 'an update was started')
  } finally {
    stop(dir)
  }
})

await sleep(500)
fs.rmSync(base, { recursive: true, force: true })
console.log(`\n${3 - failed} passed, ${failed} failed`)
process.exit(failed ? 1 : 0)
