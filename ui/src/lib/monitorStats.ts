// The readings the Monitoring section and the performance widget can show.

import * as f from './format';
import type { Monitor } from './types';

export interface Stat {
  id: string;
  label: string;
  /** Longer explanation for the settings list. */
  help: string;
  value: (m: Monitor) => string;
  /** 'warn' / 'bad' when the reading is high. */
  level?: (m: Monitor, warnCpu: number) => '' | 'warn' | 'bad';
}

const pct = (v: number) => `${v.toFixed(1)}%`;
const mb = (v: number) => (v >= 1024 ? `${(v / 1024).toFixed(2)} GB` : `${Math.round(v)} MB`);
const cpuLevel = (v: number, warn: number) => (v >= warn * 2 ? 'bad' : v >= warn ? 'warn' : '');

export const STATS: Stat[] = [
  { id: 'cpu', label: 'App CPU', help: 'Apex Overlays and all its overlay windows, % of your PC', value: (m) => pct(m.cpu), level: (m, w) => cpuLevel(m.cpu, w) },
  { id: 'cpuApp', label: 'Engine CPU', help: 'The app itself: telemetry, calculations, sending data', value: (m) => pct(m.cpuApp), level: (m, w) => cpuLevel(m.cpuApp, w) },
  { id: 'cpuWindows', label: 'Windows CPU', help: 'The overlay windows drawing (WebView2)', value: (m) => pct(m.cpuWindows), level: (m, w) => cpuLevel(m.cpuWindows, w) },
  { id: 'systemCpu', label: 'PC CPU', help: 'Your whole PC, iRacing included', value: (m) => pct(m.systemCpu) },
  { id: 'memory', label: 'Memory', help: 'Memory used by the app and its windows', value: (m) => mb(m.memoryMb) },
  { id: 'memoryApp', label: 'Engine memory', help: 'Memory of the app itself', value: (m) => mb(m.memoryAppMb) },
  { id: 'memoryWindows', label: 'Windows memory', help: 'Memory of the overlay windows', value: (m) => mb(m.memoryWindowsMb) },
  { id: 'sent', label: 'Data out', help: 'Data sent to the overlay windows and this page', value: (m) => `${m.sentKbps.toFixed(0)} kB/s` },
  { id: 'messages', label: 'Updates', help: 'Updates sent to overlay windows per second', value: (m) => `${m.messagesPerSec}/s` },
  { id: 'telemetry', label: 'Telemetry', help: 'Telemetry updates processed per second (iRacing sends 60)', value: (m) => `${Math.round(m.telemetryHz)} Hz` },
  { id: 'tick', label: 'Engine time', help: 'Time the engine spends on each telemetry update', value: (m) => `${m.tickUs.toFixed(0)} µs` },
  { id: 'clients', label: 'Windows open', help: 'Overlay windows and pages connected', value: (m) => String(m.clients) },
  { id: 'processes', label: 'Processes', help: 'The app and its window processes', value: (m) => String(m.processes) },
  { id: 'uptime', label: 'Running for', help: 'Time since Apex Overlays started', value: (m) => f.duration(m.uptime) },
];

export const statById = (id: string) => STATS.find((s) => s.id === id);
