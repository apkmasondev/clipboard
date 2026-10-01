import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { Quick } from './Quick';
import './styles.css';
class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? (
      <main className="fatal">
        <h1>Nie można wyświetlić interfejsu</h1>
        <p>Zapisane dane pozostają w magazynie. Odśwież interfejs, aby spróbować ponownie.</p>
        <button onClick={() => location.reload()}>Odśwież interfejs</button>
      </main>
    ) : (
      this.props.children
    );
  }
}
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      {new URLSearchParams(location.search).has('quick') ? <Quick /> : <App />}
    </ErrorBoundary>
  </React.StrictMode>,
);
