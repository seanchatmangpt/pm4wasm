/**
 * Collaboration types
 */

export interface SessionUser {
  id: string;
  name: string;
  color: string;
  cursor?: {
    line: number;
    column: number;
  };
  selection?: {
    start: { line: number; column: number };
    end: { line: number; column: number };
  };
}

export interface CreateSessionInput {
  name: string;
  model: string;
  isPublic?: boolean;
}

export interface SessionMessage {
  type: 'join' | 'leave' | 'update' | 'cursor' | 'selection' | 'chat';
  userId: string;
  data: unknown;
  timestamp: number;
}

export interface ModelUpdate {
  sessionId: string;
  userId: string;
  operation: 'replace' | 'insert' | 'delete';
  position: { line: number; column: number };
  content?: string;
  length?: number;
}

export interface WebrtcSignal {
  type: 'offer' | 'answer' | 'ice-candidate';
  from: string;
  to: string;
  data: RTCSessionInitInit | RTCIceCandidateInit;
}
