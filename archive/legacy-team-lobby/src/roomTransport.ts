import type { WebSocket } from 'ws';

export type RoomWireMessage = { version?: unknown; type?: unknown; [key: string]: unknown };

export type AuthenticatedRoomPeer = { socket: WebSocket };

export type RoomTransportHandlers<TPeer extends AuthenticatedRoomPeer> = {
	protocolVersion: number;
	authenticationTimeoutMs: number;
	authenticate(message: RoomWireMessage, socket: WebSocket): { peer: TPeer; history: unknown } | { error: string };
	isAuthorized(peer: TPeer): boolean;
	isCurrent(peer: TPeer): boolean;
	onAccepted(peer: TPeer): void;
	onMessage(peer: TPeer, message: RoomWireMessage): void;
	onClosed(peer: TPeer): void;
};

/** Owns WebSocket framing and the authenticated connection lifecycle for a room host. */
export function acceptRoomSocket<TPeer extends AuthenticatedRoomPeer>(socket: WebSocket, handlers: RoomTransportHandlers<TPeer>) {
	let peer: TPeer | undefined;
	const timeout = setTimeout(() => socket.close(1008, 'Authentication timeout'), handlers.authenticationTimeoutMs);
	socket.on('message', raw => {
		let message: RoomWireMessage;
		try {
			const parsed: unknown = JSON.parse(raw.toString());
			if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) { throw new Error('Message must be an object'); }
			message = parsed as RoomWireMessage;
		} catch {
			socket.close(1003, 'Invalid JSON');
			return;
		}
		if (message.version !== handlers.protocolVersion) { socket.close(1002, 'Unsupported protocol version'); return; }
		if (!peer) {
			const accepted = handlers.authenticate(message, socket);
			if ('error' in accepted) { socket.close(1008, accepted.error); return; }
			peer = accepted.peer;
			clearTimeout(timeout);
			try {
				socket.send(JSON.stringify(accepted.history));
				handlers.onAccepted(peer);
			} catch {
				socket.close(1011, 'Could not initialize room session');
			}
			return;
		}
		if (!handlers.isAuthorized(peer)) { socket.close(4003, 'Room invite revoked'); return; }
		handlers.onMessage(peer, message);
	});
	socket.on('close', () => {
		clearTimeout(timeout);
		if (peer && handlers.isCurrent(peer)) { handlers.onClosed(peer); }
	});
	socket.on('error', () => { /* close event handles removal */ });
}

