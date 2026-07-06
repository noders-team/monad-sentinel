// Type-to-confirm string for destructive operations. Operators building the
// SPA for their own node can brand it at build time:
//   VITE_NODE_NAME=my-validator npm run build
export const NODE_NAME = import.meta.env.VITE_NODE_NAME || 'monad-sentinel-node'
