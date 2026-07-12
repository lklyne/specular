import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // Contract tests spawn the real fetched agent-browser binary
    // (resources/bin/agent-browser). They skip themselves loudly when that
    // binary is absent — see tests/contract/agent-browser.contract.test.ts.
    // Not wired into CI: the fetch script is darwin-arm64-only (see
    // scripts/fetch-agent-browser.sh), so this suite is runnable locally on
    // macOS only. CI wiring is a follow-up (issue #318, D11) once a macOS
    // runner or a platform-aware fetch exists.
    include: ['tests/contract/**/*.test.ts'],
    testTimeout: 20_000,
    reporters: ['verbose'],
  },
})
