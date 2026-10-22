'use strict';

const { randomUUID } = require('node:crypto');
const { WebSocket, WebSocketServer } = require('ws');
const { authenticateAccountKey, authenticateRoomInvite } = require('./controlPlane');

const relayProtocolVersion = 1;
const maxWindowBytes = 4 * 1024 * 1024;
const windowMs = 10_000;
const participantIdPattern = /^[0-9a-f-]{36}$/i;

function parseRoomMessage(raw) {
	try { const message = JSON.parse(raw.toString()); return message && typeof message === 'object' && !Array.isArray(message) ? message : undefined; }
	catch { return undefined; }
}

function createRoomRelay({ httpsServer, store, now = () => Date.now(), id = randomUUID, maxRooms = 1000, maxParticipantsPerRoom = 32, maxConcurrentSockets = 10_000 }) {
	if (!httpsServer || !store) { throw new Error('An HTTPS server and durable room store are required.'); }
	const rooms = new Map();
	const sockets = new Set();
	const wss = new WebSocketServer({ noServer: true, maxPayload: 2 * 1024 * 1024, perMessageDeflate: false });
	function closeSocket(socket, code, reason) { try { if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) { socket.close(code, reason); } } catch { socket.terminate(); } }
	function sendHost(room, value) { if (room.host.readyState === WebSocket.OPEN) { room.host.send(JSON.stringify({ relayVersion: relayProtocolVersion, ...value })); } }
	function removeGuest(roomId, clientId, code = 1001, reason = 'Peer disconnected', notifyHost = true) {
		const room = rooms.get(roomId);
		const guest = room?.clients.get(clientId);
		if (!room || !guest) { return; }
		room.clients.delete(clientId);
		if (notifyHost) { sendHost(room, { type: 'clientDisconnect', clientId, code, reason }); }
		sockets.delete(guest.socket);
		closeSocket(guest.socket, code, reason);
	}
	function failUpgrade(socket, status) {
		try { socket.write(`HTTP/1.1 ${status}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n`); } finally { socket.destroy(); }
	}
	httpsServer.on('upgrade', (request, socket, head) => {
		let url;
		try { url = new URL(request.url, 'https://forge-room.invalid'); } catch { failUpgrade(socket, '400 Bad Request'); return; }
		const match = /^\/v1\/relay\/([0-9a-f-]{36})$/i.exec(url.pathname);
		if (!match || url.searchParams.size !== 1) { failUpgrade(socket, '404 Not Found'); return; }
		const roomId = match[1];
		const mode = url.searchParams.get('mode');
		if (mode === 'host') {
			const account = authenticateAccountKey(store, /^Bearer ([A-Za-z0-9_-]+)$/.exec(request.headers.authorization || '')?.[1]);
			const persistentRoom = store.state.rooms[roomId];
			if (!account || !persistentRoom || persistentRoom.status !== 'active' || persistentRoom.ownerAccountId !== account.id) { failUpgrade(socket, '401 Unauthorized'); return; }
			if (rooms.has(roomId) || rooms.size >= maxRooms || sockets.size >= maxConcurrentSockets) { failUpgrade(socket, rooms.has(roomId) ? '409 Conflict' : '503 Service Unavailable'); return; }
			wss.handleUpgrade(request, socket, head, host => {
				const room = { id: roomId, ownerAccountId: account.id, host, clients: new Map() };
				rooms.set(roomId, room); sockets.add(host);
				host.on('message', raw => handleHostMessage(room, raw));
				host.on('close', () => closeRoom(roomId, 1012, 'Room host disconnected'));
				host.on('error', () => closeRoom(roomId, 1011, 'Room host connection failed'));
			});
			return;
		}
		if (mode !== 'participant' || !rooms.has(roomId) || sockets.size >= maxConcurrentSockets || rooms.get(roomId).clients.size >= maxParticipantsPerRoom) { failUpgrade(socket, rooms.has(roomId) ? '503 Service Unavailable' : '404 Not Found'); return; }
		wss.handleUpgrade(request, socket, head, guestSocket => acceptGuest(roomId, guestSocket));
	});

	function acceptGuest(roomId, socket) {
		sockets.add(socket);
		const clientId = id();
		let auth;
		let closed = false;
		let bucket = { startedAt: now(), bytes: 0 };
		const timeout = setTimeout(() => closeSocket(socket, 1008, 'Room authentication timeout'), 5000);
		socket.on('message', raw => {
			if (closed) { return; }
			const time = now();
			if (time - bucket.startedAt >= windowMs) { bucket = { startedAt: time, bytes: 0 }; }
			bucket.bytes += raw.length;
			if (bucket.bytes > maxWindowBytes) { closeSocket(socket, 1008, 'Room relay rate limit exceeded'); return; }
			const room = rooms.get(roomId);
			if (!room || room.host.readyState !== WebSocket.OPEN) { closeSocket(socket, 1013, 'Room host is unavailable'); return; }
			if (!auth) {
				const hello = parseRoomMessage(raw);
				if (!hello || hello.version !== 16 || hello.type !== 'hello' || !participantIdPattern.test(hello.participantId || '') || typeof hello.name !== 'string' || !hello.name.trim() || hello.name.length > 40 || !['human', 'agent'].includes(hello.role)) { closeSocket(socket, 1008, 'Invalid room authentication'); return; }
				auth = authenticateRoomInvite(store, roomId, hello.secret, now());
				if (!auth) { closeSocket(socket, 1008, 'Invalid, expired, or revoked room invite'); return; }
				clearTimeout(timeout);
				room.clients.set(clientId, { socket, inviteId: auth.inviteId, accountId: auth.accountId, bucket });
				sendHost(room, { type: 'clientConnect', clientId, hello });
				return;
			}
			const message = roomMessageEnvelope('clientMessage', clientId, raw.toString());
			if (message) { room.host.send(message); }
		});
		socket.on('close', (code, reason) => { closed = true; clearTimeout(timeout); sockets.delete(socket); removeGuest(roomId, clientId, code || 1001, reason.toString() || 'Participant disconnected'); });
		socket.on('error', () => { /* the close event owns membership cleanup */ });
	}

	function roomMessageEnvelope(type, clientId, data) {
		if (typeof data !== 'string' || Buffer.byteLength(data, 'utf8') > 2 * 1024 * 1024) { return undefined; }
		return JSON.stringify({ relayVersion: relayProtocolVersion, type, clientId, data });
	}

	function handleHostMessage(room, raw) {
		const message = parseRoomMessage(raw);
		if (!message || message.relayVersion !== relayProtocolVersion || typeof message.clientId !== 'string') { closeSocket(room.host, 1003, 'Invalid relay message'); return; }
		const guest = room.clients.get(message.clientId);
		if (!guest) { return; }
		if (message.type === 'clientMessage' && typeof message.data === 'string' && Buffer.byteLength(message.data, 'utf8') <= 2 * 1024 * 1024) {
			if (guest.socket.readyState === WebSocket.OPEN) { guest.socket.send(message.data); }
		} else if (message.type === 'clientClose') { removeGuest(room.id, message.clientId, Number.isInteger(message.code) ? message.code : 1001, typeof message.reason === 'string' ? message.reason.slice(0, 100) : 'Room host closed participant', false); }
		else { closeSocket(room.host, 1003, 'Invalid relay message'); }
	}

	function closeRoom(roomId, code = 1012, reason = 'Room closed') {
		const room = rooms.get(roomId);
		if (!room) { return; }
		for (const [clientId, guest] of room.clients) { sockets.delete(guest.socket); closeSocket(guest.socket, code, reason); room.clients.delete(clientId); }
		rooms.delete(roomId); sockets.delete(room.host);
		closeSocket(room.host, code, reason);
	}

	return {
		webSocketServer: wss,
		rooms,
		closeRoom,
		revokeInvite(inviteId) {
			for (const room of rooms.values()) { for (const [clientId, guest] of room.clients) { if (guest.inviteId === inviteId) { removeGuest(room.id, clientId, 4003, 'Room invite revoked'); } } }
		},
		disableAccount(accountId) {
			for (const room of [...rooms.values()]) { if (room.ownerAccountId === accountId) { closeRoom(room.id, 4003, 'Room account disabled'); continue; } for (const [clientId, guest] of room.clients) { if (guest.accountId === accountId) { removeGuest(room.id, clientId, 4003, 'Room account disabled'); } } }
		},
		invalidateAccountSessions(accountId) {
			for (const room of [...rooms.values()]) { if (room.ownerAccountId === accountId) { closeRoom(room.id, 1012, 'Account key rotated; reconnect with the new key'); } }
		},
		close() { for (const roomId of [...rooms.keys()]) { closeRoom(roomId); } wss.close(); }
	};
}

module.exports = { createRoomRelay, relayProtocolVersion };
