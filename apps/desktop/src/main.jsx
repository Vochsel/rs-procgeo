import { createRoot } from 'react-dom/client';
import App from './App.jsx';

// No StrictMode: its double-invocation of effects double-cooks the graph and
// double-initialises the Three.js viewport in dev.
createRoot(document.getElementById('root')).render(<App />);
