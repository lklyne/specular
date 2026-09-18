import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { EventEmitter } from 'node:events'
import { Readable } from 'node:stream'
import type { ChildProcess } from 'node:child_process'
import {
  __resetDevServerManagerForTests,
  bindOriginToRepo,
  connectRepo,
  disconnectRepo,
  findRepoForOrigin,
  findRepoForPath,
  inferRepoPathForOrigin,
  initDevServerManager,
  listRepos,
  onChange,
  setBindingAutoFix,
  shutdownDevServerManager,
  urlForComponent,
} from '../../src/main/runtime/dev-server-manager'

interface FakeChild extends EventEmitter {
  stdout: Readable
  stderr: Readable
  kill: (signal?: string) => boolean
  killed: boolean
}

function makeFakeChild(): FakeChild {
  const child = new EventEmitter() as FakeChild
  child.stdout = new Readable({ read() {} })
  child.stderr = new Readable({ read() {} })
  child.killed = false
  child.kill = (_signal?: string) => {
    child.killed = true
    queueMicrotask(() => child.emit('exit', 0, null))
    return true
  }
  return child
}

function emitLine(stream: Readable, line: string): void {
  stream.push(`${line}\n`)
}

describe('dev-server-manager', () => {
  let dir: string
  let spawnCalls: Array<{ command: string; args: readonly string[]; cwd: string }>
  let nextChild: FakeChild | null
  let pendingChildren: FakeChild[]

  beforeEach(() => {
    __resetDevServerManagerForTests()
    dir = mkdtempSync(join(tmpdir(), 'specular-dev-server-'))
    spawnCalls = []
    nextChild = null
    pendingChildren = []
    initDevServerManager({
      userDataDir: dir,
      spawn: (command, args, options) => {
        spawnCalls.push({ command, args, cwd: options.cwd })
        const child = nextChild ?? makeFakeChild()
        nextChild = null
        pendingChildren.push(child)
        return child as unknown as ChildProcess
      },
    })
  })

  afterEach(async () => {
    await shutdownDevServerManager()
    rmSync(dir, { recursive: true, force: true })
  })

  it('connectRepo persists to repos.json and is idempotent by path', () => {
    const a = connectRepo('/abs/path/to/my-repo')
    const b = connectRepo('/abs/path/to/my-repo', 'ignored-relabel')
    expect(a.id).toBe(b.id)
    expect(a.label).toBe('my-repo')
    expect(listRepos()).toHaveLength(1)

    const persisted = JSON.parse(readFileSync(join(dir, 'repos.json'), 'utf8'))
    expect(persisted.repos).toEqual([
      { id: a.id, absolutePath: '/abs/path/to/my-repo', label: 'my-repo' },
    ])
  })

  it('persisted repos are reloaded on init', () => {
    const repo = connectRepo('/abs/x/repo-a')
    __resetDevServerManagerForTests()
    initDevServerManager({
      userDataDir: dir,
      spawn: () => makeFakeChild() as unknown as ChildProcess,
    })
    const reloaded = listRepos()
    expect(reloaded).toHaveLength(1)
    expect(reloaded[0].id).toBe(repo.id)
    expect(reloaded[0].absolutePath).toBe('/abs/x/repo-a')
  })

  it('urlForComponent spawns vite, parses Local: URL, and returns a __specular URL', async () => {
    const repo = connectRepo('/abs/path/repo-x')
    const child = makeFakeChild()
    nextChild = child

    const promise = urlForComponent(repo.id, 'src/Button.tsx')
    queueMicrotask(() => {
      emitLine(child.stdout, '  ➜  Local:   http://localhost:5173/')
    })
    const url = await promise

    expect(url).toBe('http://localhost:5173/__specular?path=src%2FButton.tsx')
    expect(spawnCalls[0]).toMatchObject({
      command: 'npx',
      args: ['vite', 'dev'],
      cwd: '/abs/path/repo-x',
    })

    const status = listRepos().find((r) => r.id === repo.id)
    expect(status?.status).toBe('running')
    expect(status?.port).toBe(5173)
    expect(status?.baseUrl).toBe('http://localhost:5173')
  })

  it('multiple urlForComponent calls share one child process', async () => {
    const repo = connectRepo('/abs/share')
    const child = makeFakeChild()
    nextChild = child

    const first = urlForComponent(repo.id, 'a.tsx')
    const second = urlForComponent(repo.id, 'b.tsx')
    queueMicrotask(() => {
      emitLine(child.stdout, 'Local:   http://localhost:5179/')
    })
    const [a, b] = await Promise.all([first, second])

    expect(a).toContain('http://localhost:5179/__specular?path=a.tsx')
    expect(b).toContain('http://localhost:5179/__specular?path=b.tsx')
    expect(spawnCalls).toHaveLength(1)
  })

  it('disconnectRepo kills the child and removes the repo', async () => {
    const repo = connectRepo('/abs/disc')
    const child = makeFakeChild()
    nextChild = child
    const promise = urlForComponent(repo.id, 'a.tsx')
    queueMicrotask(() => emitLine(child.stdout, 'Local:   http://localhost:5180/'))
    await promise

    await disconnectRepo(repo.id)
    expect(child.killed).toBe(true)
    expect(listRepos()).toHaveLength(0)
  })

  it('child exit flips status back to stopped', async () => {
    const repo = connectRepo('/abs/exit')
    const child = makeFakeChild()
    nextChild = child
    const promise = urlForComponent(repo.id, 'a.tsx')
    queueMicrotask(() => emitLine(child.stdout, 'Local:   http://localhost:5181/'))
    await promise

    child.emit('exit', 0, null)
    const after = listRepos().find((r) => r.id === repo.id)
    expect(after?.status).toBe('stopped')
    expect(after?.baseUrl).toBeNull()
  })

  it('auto-bind steals an origin already bound to another repo', async () => {
    const repoA = connectRepo('/abs/steal/repo-a')
    const childA = makeFakeChild()
    nextChild = childA
    const startA = urlForComponent(repoA.id, 'a.tsx')
    queueMicrotask(() => emitLine(childA.stdout, 'Local:   http://localhost:5182/'))
    await startA
    expect(findRepoForOrigin('http://localhost:5182')?.id).toBe(repoA.id)

    childA.emit('exit', 0, null)

    const repoB = connectRepo('/abs/steal/repo-b')
    const childB = makeFakeChild()
    nextChild = childB
    const startB = urlForComponent(repoB.id, 'b.tsx')
    queueMicrotask(() => emitLine(childB.stdout, 'Local:   http://localhost:5182/'))
    await startB

    expect(findRepoForOrigin('http://localhost:5182')?.id).toBe(repoB.id)
    const staleA = listRepos().find((r) => r.id === repoA.id)
    expect(staleA?.boundOrigins).toHaveLength(0)
  })

  it('auto-bind keeps the autoFix flag across restarts of the same repo', async () => {
    const repo = connectRepo('/abs/keep-flag')
    const first = makeFakeChild()
    nextChild = first
    const start = urlForComponent(repo.id, 'a.tsx')
    queueMicrotask(() => emitLine(first.stdout, 'Local:   http://localhost:5183/'))
    await start
    setBindingAutoFix('http://localhost:5183', true)

    first.emit('exit', 0, null)
    const second = makeFakeChild()
    nextChild = second
    const restart = urlForComponent(repo.id, 'b.tsx')
    queueMicrotask(() => emitLine(second.stdout, 'Local:   http://localhost:5183/'))
    await restart

    const binding = listRepos()
      .find((r) => r.id === repo.id)
      ?.boundOrigins.find((b) => b.origin === 'http://localhost:5183')
    expect(binding?.autoFix).toBe(true)
  })

  it('findRepoForPath matches repo by exact path or by parent', () => {
    const repo = connectRepo('/abs/match/repo')
    expect(findRepoForPath('/abs/match/repo')?.id).toBe(repo.id)
    expect(findRepoForPath('/abs/match/repo/src/Button.tsx')?.id).toBe(repo.id)
    expect(findRepoForPath('/abs/match/other')).toBeNull()
  })

  it('onChange notifies listeners on connect/disconnect', () => {
    const events: number[] = []
    const off = onChange((repos) => events.push(repos.length))
    connectRepo('/abs/notify')
    expect(events.at(-1)).toBe(1)
    off()
  })

  it('inferRepoPathForOrigin uses an explicit origin binding', () => {
    const repo = connectRepo('/abs/bound/repo')
    bindOriginToRepo(repo.id, 'http://localhost:4321')
    expect(inferRepoPathForOrigin('http://localhost:4321')).toBe('/abs/bound/repo')
    expect(inferRepoPathForOrigin('http://localhost:9999')).toBeNull()
  })
})
