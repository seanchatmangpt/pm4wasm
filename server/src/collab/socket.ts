/**
 * WebSocket server for real-time collaboration
 */

import { Server as HTTPServer } from 'http';
import { Server as SocketIOServer } from 'socket.io';
import { prisma } from '../db/prisma.js';
import { collabService } from './service.js';
import type { Socket } from 'socket.io';
import type { DefaultEventsMap } from 'socket.io/dist/typed-events';

interface ServerToClientEvents {
  'user:joined': (user: SessionUser) => void;
  'user:left': (userId: string) => void;
  'model:updated': (update: ModelUpdate) => void;
  'cursor:moved': (userId: string, cursor: { line: number; column: number }) => void;
  'selection:changed': (userId: string, selection: { start: { line: number; column: number }; end: { line: number; column: number } }) => void;
  'signal': (signal: WebrtcSignal) => void;
}

interface ClientToServerEvents {
  'join:session': (ticket: string) => void;
  'update:model': (update: ModelUpdate) => void;
  'move:cursor': (cursor: { line: number; column: number }) => void;
  'change:selection': (selection: { start: { line: number; column: number }; end: { line: number; column: number } }) => void;
  'send:signal': (signal: WebrtcSignal) => void;
}

interface SessionUser {
  id: string;
  name: string;
  color: string;
  cursor?: { line: number; column: number };
  selection?: { start: { line: number; column: number }; end: { line: number; column: number } };
}

interface ModelUpdate {
  sessionId: string;
  userId: string;
  operation: 'replace' | 'insert' | 'delete';
  position: { line: number; column: number };
  content?: string;
  length?: number;
}

interface WebrtcSignal {
  type: 'offer' | 'answer' | 'ice-candidate';
  from: string;
  to: string;
  data: RTCSessionInitInit | RTCIceCandidateInit;
}

// Store active users per session
const sessionUsers = new Map<string, Map<string, SessionUser>>();

// Store socket-to-session mapping
const socketToSession = new Map<Socket, string>();

// Color palette for users
const USER_COLORS = [
  '#FF6B6B', '#4ECDC4', '#45B7D1', '#FFA07A', '#98D8C8',
  '#F7DC6F', '#BB8FCE', '#85C1E2', '#F8B500', '#00CED1',
];

function getUserColor(userId: string): string {
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = userId.charCodeAt(i) + ((hash << 5) - hash);
  }
  return USER_COLORS[Math.abs(hash) % USER_COLORS.length];
}

export function createWebSocketServer(httpServer: HTTPServer) {
  const io = new SocketIOServer<ClientToServerEvents, ServerToClientEvents, DefaultEventsMap, DefaultEventsMap>(httpServer, {
    path: '/socket.io/',
    cors: {
      origin: process.env.CORS_ORIGIN || 'http://localhost:5173',
      credentials: true,
    },
    transports: ['websocket', 'polling'],
  });

  io.on('connection', (socket) => {
    console.log(`Client connected: ${socket.id}`);

    socket.on('join:session', async (ticket) => {
      try {
        const validated = collabService.validateJoinTicket(ticket);
        if (!validated) {
          socket.emit('error', { message: 'Invalid or expired ticket' });
          return;
        }

        const { sessionId, userId } = validated;

        // Verify session exists
        const session = await collabService.getSession(sessionId);
        if (!session) {
          socket.emit('error', { message: 'Session not found' });
          return;
        }

        // Get user info
        const user = await prisma.user.findUnique({ where: { id: userId } });
        if (!user) {
          socket.emit('error', { message: 'User not found' });
          return;
        }

        // Join session room
        socket.join(sessionId);
        socketToSession.set(socket, sessionId);

        // Add to active users
        if (!sessionUsers.has(sessionId)) {
          sessionUsers.set(sessionId, new Map());
        }

        const sessionUser: SessionUser = {
          id: userId,
          name: user.name,
          color: getUserColor(userId),
        };
        sessionUsers.get(sessionId)!.set(userId, sessionUser);

        // Notify others
        socket.to(sessionId).emit('user:joined', sessionUser);

        // Send current users list to joiner
        const users = Array.from(sessionUsers.get(sessionId)!.values());
        socket.emit('session:users', users);

        // Send current model
        socket.emit('model:current', session.model);

        console.log(`User ${userId} joined session ${sessionId}`);
      } catch (error) {
        console.error('Error joining session:', error);
        socket.emit('error', { message: 'Failed to join session' });
      }
    });

    socket.on('update:model', async (update) => {
      const sessionId = socketToSession.get(socket);
      if (!sessionId) return;

      try {
        // Record update in history
        await collabService.recordUpdate(sessionId, socket.id, update);

        // Broadcast to others in session
        socket.to(sessionId).emit('model:updated', update);
      } catch (error) {
        console.error('Error updating model:', error);
      }
    });

    socket.on('move:cursor', (cursor) => {
      const sessionId = socketToSession.get(socket);
      if (!sessionId) return;

      // Update user's cursor in store
      const users = sessionUsers.get(sessionId);
      if (users) {
        const user = Array.from(users.values()).find(u => u.id === socket.id);
        if (user) {
          user.cursor = cursor;
        }
      }

      // Broadcast to others
      socket.to(sessionId).emit('cursor:moved', socket.id, cursor);
    });

    socket.on('change:selection', (selection) => {
      const sessionId = socketToSession.get(socket);
      if (!sessionId) return;

      // Update user's selection in store
      const users = sessionUsers.get(sessionId);
      if (users) {
        const user = Array.from(users.values()).find(u => u.id === socket.id);
        if (user) {
          user.selection = selection;
        }
      }

      // Broadcast to others
      socket.to(sessionId).emit('selection:changed', socket.id, selection);
    });

    socket.on('send:signal', (signal) => {
      const sessionId = socketToSession.get(socket);
      if (!sessionId) return;

      // Relay WebRTC signal to target user
      socket.to(signal.to).emit('signal', {
        ...signal,
        from: socket.id,
      });
    });

    socket.on('disconnect', () => {
      const sessionId = socketToSession.get(socket);
      if (sessionId) {
        // Remove from active users
        const users = sessionUsers.get(sessionId);
        if (users) {
          users.delete(socket.id);

          // Notify others
          io.to(sessionId).emit('user:left', socket.id);
        }

        socketToSession.delete(socket);
        console.log(`Client ${socket.id} disconnected from session ${sessionId}`);
      }
    });
  });

  return io;
}

export { sessionUsers, socketToSession };
