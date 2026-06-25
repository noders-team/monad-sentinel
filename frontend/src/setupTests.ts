import '@testing-library/jest-dom'
import { server } from './test/mswServer'

beforeAll(() => server.listen({ onUnhandledRequest: 'bypass' }))
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
