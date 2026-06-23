import { Studio } from '@procgeo/studio';
import '@procgeo/studio/styles.css';
import { nativeEngine } from './engine.js';

export default function App() {
  return <Studio engine={nativeEngine} />;
}
