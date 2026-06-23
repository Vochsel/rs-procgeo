import { useEffect, useRef } from 'react';
import * as monaco from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

// Vite ESM worker wiring (shared by every app that mounts the studio).
if (!self.MonacoEnvironment) {
  self.MonacoEnvironment = {
    getWorker(_id, label) {
      if (label === 'typescript' || label === 'javascript') return new tsWorker();
      return new editorWorker();
    },
  };
}

/**
 * Monaco editor bound to the round-trip DSL. `value` is pushed in only when it
 * differs from the editor's current text (so graph edits don't fight the caret).
 */
export function CodeEditor({ value, onChange }) {
  const mountRef = useRef(null);
  const editorRef = useRef(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
      noSemanticValidation: true,
      noSyntaxValidation: true,
    });

    const editor = monaco.editor.create(mountRef.current, {
      value: value || '',
      language: 'javascript',
      theme: 'vs-dark',
      fontSize: 13,
      minimap: { enabled: false },
      automaticLayout: true,
      scrollBeyondLastLine: false,
      padding: { top: 10 },
      tabSize: 2,
    });
    editorRef.current = editor;

    const sub = editor.onDidChangeModelContent(() => {
      onChangeRef.current?.(editor.getValue());
    });

    return () => {
      sub.dispose();
      editor.dispose();
      editorRef.current = null;
    };
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && value !== undefined && value !== editor.getValue()) {
      editor.setValue(value);
    }
  }, [value]);

  return <div className="pg-code" ref={mountRef} />;
}
