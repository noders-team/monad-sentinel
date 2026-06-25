import '@testing-library/jest-dom'
import { server } from './test/mswServer'

globalThis.ResizeObserver = class { observe() {}; unobserve() {}; disconnect() {} }

beforeAll(() => server.listen({ onUnhandledRequest: 'bypass' }))
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
