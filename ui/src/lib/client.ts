// WebSocket client: subscribes to topics, reconnects automatically and
// batches incoming data so consumers render at most once per animation frame.

import type { TopicMap, TopicName } from './types';

type Handler = (topic: TopicName, data: any) => void;

export class Client {
  private ws: WebSocket | null = null;
  private topics = new Set<string>();
  private pending = new Map<TopicName, unknown>();
  private frame = 0;
  readonly latest: Partial<TopicMap> = {};
  connected = false;

  constructor(private onData: Handler, private onConnection?: (up: boolean) => void) {
    this.open();
  }

  private url(): string {
    const q = new URLSearchParams(location.search);
    const host = q.get('server') ?? location.host;
    return `${location.protocol === 'https:' ? 'wss' : 'ws'}://${host}/ws`;
  }

  private open() {
    const ws = new WebSocket(this.url());
    this.ws = ws;
    ws.onopen = () => {
      this.connected = true;
      this.onConnection?.(true);
      if (this.topics.size) this.send({ op: 'sub', topics: [...this.topics] });
    };
    ws.onmessage = (ev) => {
      const msg = JSON.parse(ev.data as string) as { t: TopicName; d: unknown };
      this.pending.set(msg.t, msg.d);
      (this.latest as Record<string, unknown>)[msg.t] = msg.d;
      if (!this.frame) this.frame = requestAnimationFrame(() => this.flush());
    };
    ws.onclose = () => {
      this.connected = false;
      this.onConnection?.(false);
      setTimeout(() => this.open(), 1000);
    };
  }

  private flush() {
    this.frame = 0;
    const batch = [...this.pending];
    this.pending.clear();
    for (const [t, d] of batch) this.onData(t, d);
  }

  subscribe(...topics: string[]) {
    const fresh = topics.filter((t) => !this.topics.has(t));
    fresh.forEach((t) => this.topics.add(t));
    if (fresh.length && this.ws?.readyState === WebSocket.OPEN) this.send({ op: 'sub', topics: fresh });
  }

  send(msg: object) {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg));
  }
}
