/**
 * Collaboration client
 */

import { io, Socket } from 'socket.io-client';
import type { Session, CreateSessionInput, SessionUser } from './types.js';

export interface CollabEvents {
  'user:joined': (user: SessionUser) => void;
  'user:left': (userId: string) => void;
  'model:updated': (update: ModelUpdate) => void;
  'cursor:moved': (userId: string, cursor: { line: number; column: number }) => void;
  'selection:changed': (userId: string, selection: Selection) => void;
  'signal': (signal: WebrtcSignal) => void;
  'error': (error: { message: string }) => void;
}

export interface ModelUpdate {
  sessionId: string;
  userId: string;
  operation: 'replace' | 'insert' | 'delete';
  position: { line: number; column: number };
  content?: string;
  length?: number;
}

export interface Selection {
  start: { line: number; column: number };
  end: { line: number; column: number };
}

export interface WebrtcSignal {
  type: 'offer' | 'answer' | 'ice-candidate';
  from: string;
  to: string;
  data: RTCSessionInitInit | RTCIceCandidateInit;
}

export class CollabClient {
  private baseUrl: string;
  private getAccessToken: () => Promise<string>;
  private socket: Socket | null = null;
  private currentSession: string | null = null;

  constructor(baseUrl: string, getAccessToken: () => Promise<string>) {
    this.baseUrl = baseUrl;
    this.getAccessToken = getAccessToken;
  }

  /**
   * List sessions
   */
  async list(tenantId: string): Promise<Session[]> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions?tenantId=${tenantId}`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('LIST_SESSIONS_FAILED');
    }

    const data = await response.json();
    return data.sessions;
  }

  /**
   * Get a session
   */
  async get(sessionId: string): Promise<Session> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions/${sessionId}`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('GET_SESSION_FAILED');
    }

    return response.json();
  }

  /**
   * Create a new session
   */
  async create(input: CreateSessionInput): Promise<Session> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(input),
    });

    if (!response.ok) {
      throw new Error('CREATE_SESSION_FAILED');
    }

    return response.json();
  }

  /**
   * Delete a session
   */
  async delete(sessionId: string): Promise<void> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions/${sessionId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('DELETE_SESSION_FAILED');
    }
  }

  /**
   * Join a collaborative session
   */
  async join(sessionId: string, events: CollabEvents): Promise<void> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions/${sessionId}/join`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('JOIN_SESSION_FAILED');
    }

    const { ticket, wsUrl } = await response.json();

    // Connect to WebSocket
    this.socket = io(wsUrl, {
      path: '/socket.io/',
      auth: { ticket },
      transports: ['websocket'],
    });

    this.currentSession = sessionId;

    // Set up event listeners
    this.socket.on('connect', () => {
      console.log('Connected to collaboration session');
    });

    this.socket.on('session:users', (users: SessionUser[]) => {
      if (events['user:joined']) {
        // Emit join event for each existing user
        users.forEach(user => events['user:joined']!(user));
      }
    });

    this.socket.on('user:joined', events['user:joined']);
    this.socket.on('user:left', events['user:left']);
    this.socket.on('model:updated', events['model:updated']);
    this.socket.on('cursor:moved', events['cursor:moved']);
    this.socket.on('selection:changed', events['selection:changed']);
    this.socket.on('signal', events['signal']);
    this.socket.on('error', events['error']);

    // Wait for connection
    await new Promise<void>((resolve) => {
      this.socket!.on('connect', resolve);
    });
  }

  /**
   * Leave the current session
   */
  leave(): void {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
      this.currentSession = null;
    }
  }

  /**
   * Send model update
   */
  sendModelUpdate(update: Omit<ModelUpdate, 'sessionId' | 'userId'>): void {
    if (!this.socket || !this.currentSession) {
      throw new Error('NOT_CONNECTED');
    }

    this.socket.emit('update:model', {
      ...update,
      sessionId: this.currentSession,
    });
  }

  /**
   * Send cursor movement
   */
  moveCursor(cursor: { line: number; column: number }): void {
    if (!this.socket) {
      throw new Error('NOT_CONNECTED');
    }

    this.socket.emit('move:cursor', cursor);
  }

  /**
   * Send selection change
   */
  changeSelection(selection: Selection): void {
    if (!this.socket) {
      throw new Error('NOT_CONNECTED');
    }

    this.socket.emit('change:selection', selection);
  }

  /**
   * Send WebRTC signal
   */
  sendSignal(signal: Omit<WebrtcSignal, 'from'>): void {
    if (!this.socket) {
      throw new Error('NOT_CONNECTED');
    }

    this.socket.emit('send:signal', {
      ...signal,
      from: this.socket.id,
    });
  }

  /**
   * Get session history
   */
  async getHistory(sessionId: string): Promise<Array<{ userId: string; action: string; data: unknown; timestamp: Date }>> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/sessions/${sessionId}/history`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('GET_HISTORY_FAILED');
    }

    const data = await response.json();
    return data.history;
  }
}
