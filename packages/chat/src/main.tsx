import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './app';
import './index.css';

// Initialize theme from localStorage
const stored = localStorage.getItem('crewride-theme');
if (stored === 'dark' || (!stored && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
    document.documentElement.classList.add('dark');
}

ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
        <App />
    </React.StrictMode>,
);
