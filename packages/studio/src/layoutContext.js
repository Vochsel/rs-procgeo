import { createContext } from 'react';

// Node-graph flow direction: 'LR' (left→right) or 'TB' (top→bottom).
export const DirectionContext = createContext('LR');

// Per-node flag actions (Houdini-style display / bypass flags).
export const NodeActionsContext = createContext({
  outputId: null,
  onSetOutput: () => {},
  onToggleBypass: () => {},
  onRename: () => {},
});
