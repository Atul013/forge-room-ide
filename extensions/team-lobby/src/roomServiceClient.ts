import { WebSocket } from 'ws';

const serviceRoomIdPattern = /^[0-9a-f-]{36}$/i;
const serviceInvitePattern = /^fr_invite_[A-Za-z0-9_-]{43}$/;
const serviceAccountPattern = /^fr_account_[A-Za-z0-9_-]{43}$/;
const bridgeParticipantPattern = /^[0-9a-f-]{36}$/i;
const maxRelayMessageBytes = 2 * 1024 * 1024;
const maxPendingMessages = 64;

export function normalizeRoomServiceUrl(value: string): string | undefined {
	try {
		const url = new URL(value.trim());
		if (url.protocol !== 'https:' || !url.hostname || url.username || url.password || url.pathname !== '/' || url.search || url.hash) { return undefined; }
		return url.origin;
	} catch { return undefined; }
}

export function roomServiceRelayUrl(baseUrl: string, roomId: string, role: 'host' | 'participant'): string | undefined {
	const base = normalizeRoomServiceUrl(baseUrl);
	if (!base || !serviceRoomIdPattern.test(roomId)) { return undefined; }
	const url = new URL(`/v1/relay/${roomId}`, base);
	url.protocol = 'wss:';
	url.searchParams.set('mode', role);
	return url.toString();
}

export async function callRoomService<T>(baseUrl: string, accountKey: string, route: string, method = 'GET', body?: unknown): Promise<T> {
	const base = normalizeRoomServiceUrl(baseUrl);
	if (!base || !serviceAccountPattern.test(accountKey) || !route.startsWith('/v1/') || route.includes('..') || /[\r\n]/.test(route)) { throw new Error('Room service settings or route are invalid.'); }
	const response = await fetch(new URL(route, base), {
		method,
		headers: { authorization: `Bearer ${accountKey}`, ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
		...(body === undefined ? {} : { body: JSON.stringify(body) }),
		redirect: 'error',
		signal: AbortSignal.timeout(10_000)
	});
	const maxResponseBytes = 64 * 1024;
	if (Number(response.headers.get('content-length')) > maxResponseBytes) { throw new Error('Room service response exceeded the 64 KB limit.'); }
	const reader = response.body?.getReader();
	const chunks: Uint8Array[] = [];
	let responseBytes = 0;
	if (reader) {
		while (true) {
			const { done, value } = await reader.read();
			if (done) { break; }
			responseBytes += value.byteLength;
			if (responseBytes > maxResponseBytes) { await reader.cancel(); throw new Error('Room service response exceeded the 64 KB limit.'); }
			chunks.push(value);
		}
	}
	const text = new TextDecoder().decode(Buffer.concat(chunks.map(chunk => Buffer.from(chunk))));
	let result: any;
	try { result = JSON.parse(text); } catch { throw new Error(`Room service returned an invalid response (${response.status}).`); }
	if (!response.ok) {
		const code = typeof result?.error === 'string' && /^[a-z_]{1,60}$/.test(result.error) ? result.error : 'request_failed';
		throw new Error(`Room service request failed (${response.status}: ${code}).`);
	}
	return result as T;
}

type RelayEnvelope = { relayVersion: 1; type: 'clientConnect' | 'clientMessage' | 'clientDisconnect'; clientId: string; hello?: Record<string, unknown>; data?: string; code?: number; reason?: string };
type BridgePeer = { socket: WebSocket; opened: boolean; queue: string[]; queueBytes: number };

/** Bridges service-authenticated guest sockets to the existing local room authority. */
export class RoomServiceTunnel {
	private readonly relay: WebSocket;
	private readonly peers = new Map<string, BridgePeer>();
	private finished = false;
	private connectionError?: Error;

	constructor(
		private readonly relayUrl: string,
		accountKey: string,
		private readonly internalInvite: string,
		private readonly localRoomUrl: string,
		private readonly onDisconnect: (reason: string) => void = () => undefined,
		trustedCa?: Buffer
	) {
		if (!serviceAccountPattern.test(accountKey) || !/^(ws|wss):\/\//.test(localRoomUrl)) { throw new Error('Invalid room relay credentials or local room address.'); }
		const remote = new URL(relayUrl);
		if (remote.protocol !== 'wss:' || remote.searchParams.get('mode') !== 'host' || !serviceRoomIdPattern.test(remote.pathname.split('/').at(-1) || '') || remote.searchParams.size !== 1) { throw new Error('Room service relay must use a validated WSS host endpoint.'); }
		if (!/^[A-Za-z0-9_-]{43}$/.test(internalInvite)) { throw new Error('Internal room invitation is invalid.'); }
		this.relay = new WebSocket(relayUrl, { headers: { Authorization: `Bearer ${accountKey}` }, maxPayload: maxRelayMessageBytes, perMessageDeflate: false, ...(trustedCa ? { ca: trustedCa } : {}) });
		this.relay.on('message', raw => this.handleRelayMessage(raw));
		this.relay.on('close', (_code, reason) => { if (!this.finished) { this.finished = true; this.closePeers(); this.onDisconnect(reason.toString() || 'Room service connection closed.'); } });
		this.relay.on('error', error => { this.connectionError = error; if (this.relay.readyState === WebSocket.CONNECTING) { this.closePeers(); } });
	}

	connect(): Promise<void> {
		if (this.relay.readyState === WebSocket.OPEN) { return Promise.resolve(); }
		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => { cleanup(); this.relay.terminate(); reject(new Error('Room service connection timed out.')); }, 10_000);
			const onOpen = () => { cleanup(); resolve(); };
			const onError = (error: Error) => { cleanup(); reject(this.connectionError || error); };
			const cleanup = () => { clearTimeout(timeout); this.relay.off('open', onOpen); this.relay.off('error', onError); };
			this.relay.once('open', onOpen);
			this.relay.once('error', onError);
		});
	}

	private handleRelayMessage(raw: Buffer | ArrayBuffer | Buffer[]) {
		if (raw instanceof Buffer && raw.length > maxRelayMessageBytes) { this.relay.close(1009, 'Relay message too large'); return; }
		let message: RelayEnvelope;
		try { message = JSON.parse(raw.toString()) as RelayEnvelope; } catch { this.relay.close(1003, 'Invalid relay JSON'); return; }
		if (!message || message.relayVersion !== 1 || !bridgeParticipantPattern.test(message.clientId || '')) { this.relay.close(1008, 'Invalid relay envelope'); return; }
		if (message.type === 'clientConnect') { this.connectPeer(message); return; }
		const peer = this.peers.get(message.clientId);
		if (!peer) { return; }
		if (message.type === 'clientDisconnect') { this.closePeer(message.clientId, message.code || 1001, message.reason || 'Room participant disconnected'); return; }
		if (message.type !== 'clientMessage' || typeof message.data !== 'string' || Buffer.byteLength(message.data, 'utf8') > maxRelayMessageBytes) { this.relay.close(1008, 'Invalid relay message'); return; }
		if (peer.opened && peer.socket.readyState === WebSocket.OPEN) { peer.socket.send(message.data); return; }
		peer.queueBytes += Buffer.byteLength(message.data, 'utf8');
		if (peer.queue.length >= maxPendingMessages || peer.queueBytes > maxRelayMessageBytes) { this.closePeer(message.clientId, 1008, 'Room peer message queue exceeded its limit'); return; }
		peer.queue.push(message.data);
	}

	private connectPeer(message: RelayEnvelope) {
		if (this.peers.has(message.clientId) || !message.hello || message.hello.type !== 'hello' || message.hello.version !== 16 || !bridgeParticipantPattern.test(String(message.hello.participantId || '')) || typeof message.hello.name !== 'string' || message.hello.name.length > 40 || !['human', 'agent'].includes(String(message.hello.role))) { this.relay.close(1008, 'Invalid or duplicate room participant'); return; }
		const socket = new WebSocket(this.localRoomUrl, { maxPayload: maxRelayMessageBytes, perMessageDeflate: false });
		const peer: BridgePeer = { socket, opened: false, queue: [], queueBytes: 0 };
		this.peers.set(message.clientId, peer);
		socket.on('open', () => {
			if (this.peers.get(message.clientId) !== peer) { socket.close(); return; }
			peer.opened = true;
			const hello = { ...message.hello, secret: this.internalInvite };
			socket.send(JSON.stringify(hello));
			for (const data of peer.queue) { if (socket.readyState === WebSocket.OPEN) { socket.send(data); } }
			peer.queue = []; peer.queueBytes = 0;
		});
		socket.on('message', data => {
			if (this.relay.readyState !== WebSocket.OPEN || this.peers.get(message.clientId) !== peer) { return; }
			this.relay.send(JSON.stringify({ relayVersion: 1, type: 'clientMessage', clientId: message.clientId, data: data.toString() }));
		});
		socket.on('close', (code, reason) => {
			if (this.peers.get(message.clientId) !== peer) { return; }
			this.peers.delete(message.clientId);
			if (this.relay.readyState === WebSocket.OPEN) { this.relay.send(JSON.stringify({ relayVersion: 1, type: 'clientClose', clientId: message.clientId, code: code || 1001, reason: reason.toString().slice(0, 100) })); }
		});
		socket.on('error', () => { /* close event removes the local bridge */ });
	}

	private closePeer(clientId: string, code: number, reason: string) {
		const peer = this.peers.get(clientId);
		if (!peer) { return; }
		this.peers.delete(clientId);
		try { peer.socket.close(code, reason.slice(0, 100)); } catch { peer.socket.terminate(); }
	}

	private closePeers() { for (const peer of this.peers.values()) { peer.socket.terminate(); } this.peers.clear(); }
	close() { this.finished = true; this.closePeers(); this.relay.close(1000, 'Room host stopped'); }
}
