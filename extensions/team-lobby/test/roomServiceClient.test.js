'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const { once } = require('node:events');
const { randomUUID, createHash } = require('node:crypto');
const { WebSocket, WebSocketServer } = require('ws');
const { normalizeRoomServiceUrl, roomServiceRelayUrl, RoomServiceTunnel } = require('../dist/roomServiceClient.js');
const { JsonRoomStore, createRoomControlPlane } = require('../../../services/room-service/src/controlPlane');
const { createRoomRelay } = require('../../../services/room-service/src/relayServer');

test('room service accepts only HTTPS origins and emits WSS relay endpoints', () => {
	assert.equal(normalizeRoomServiceUrl('https://rooms.example.test'), 'https://rooms.example.test');
	assert.equal(normalizeRoomServiceUrl('http://rooms.example.test'), undefined);
	assert.equal(normalizeRoomServiceUrl('https://user:pass@rooms.example.test'), undefined);
	assert.equal(normalizeRoomServiceUrl('https://rooms.example.test/api'), undefined);
	const id = '123e4567-e89b-42d3-a456-426614174000';
	assert.equal(roomServiceRelayUrl('https://rooms.example.test', id, 'participant'), `wss://rooms.example.test/v1/relay/${id}?mode=participant`);
	assert.equal(roomServiceRelayUrl('https://rooms.example.test', 'bad-id', 'host'), undefined);
});

test('room service tunnel rejects malformed credentials and non-host relay URLs', () => {
	const key = `fr_account_${'a'.repeat(43)}`;
	const id = '123e4567-e89b-42d3-a456-426614174000';
	const localInvite = 'a'.repeat(43);
	assert.throws(() => new RoomServiceTunnel(`wss://rooms.example.test/v1/relay/${id}?mode=participant`, key, localInvite, 'ws://127.0.0.1:4777'), /validated WSS host endpoint/);
	assert.throws(() => new RoomServiceTunnel(`wss://rooms.example.test/v1/relay/${id}?mode=host`, key, 'fr_invite_invalid', 'ws://127.0.0.1:4777'), /Internal room invitation is invalid/);
	assert.throws(() => new RoomServiceTunnel(`wss://rooms.example.test/v1/relay/${id}?mode=host`, 'wrong', localInvite, 'ws://127.0.0.1:4777'), /Invalid room relay credentials/);
});

test('extension room tunnel bridges an invite-authenticated WSS guest to the local room authority', async t => {
	const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-extension-relay-'));
	const cert = fs.readFileSync(path.join(__dirname, '../../../services/room-service/test/fixtures/server-cert.pem'));
	const pfx = fs.readFileSync(path.join(__dirname, '../../../services/room-service/test/fixtures/server.pfx'));
	const store = new JsonRoomStore(path.join(directory, 'state.json'));
	const accountId = randomUUID();
	const roomId = randomUUID();
	const inviteId = randomUUID();
	const accountKey = `fr_account_${'A'.repeat(43)}`;
	const inviteToken = `fr_invite_${'B'.repeat(43)}`;
	const internalInvite = 'C'.repeat(43);
	const createdAt = new Date().toISOString();
	store.commit(state => {
		state.accounts[accountId] = { id: accountId, name: 'Owner', status: 'active', keyHash: createHash('sha256').update(accountKey).digest('hex'), createdAt };
		state.rooms[roomId] = { id: roomId, ownerAccountId: accountId, name: 'Bridge test room', status: 'active', createdAt };
		state.invites[inviteId] = { id: inviteId, roomId, createdBy: accountId, label: 'Bridge test', tokenHash: createHash('sha256').update(inviteToken).digest('hex'), createdAt, expiresAt: new Date(Date.now() + 60_000).toISOString(), revokedAt: null };
	});
	const handle = createRoomControlPlane({ store, adminKey: 'a'.repeat(48) });
	const service = https.createServer({ pfx, passphrase: 'test-only-not-for-deployment', minVersion: 'TLSv1.2' }, (request, response) => { void handle(request, response); });
	const relay = createRoomRelay({ httpsServer: service, store });
	const authority = new WebSocketServer({ host: '127.0.0.1', port: 0, perMessageDeflate: false });
	await Promise.all([
		new Promise((resolve, reject) => { service.once('error', reject); service.listen(0, '127.0.0.1', resolve); }),
		new Promise((resolve, reject) => { authority.once('error', reject); authority.once('listening', resolve); })
	]);
	const localPort = authority.address().port;
	const relayUrl = `wss://localhost:${service.address().port}/v1/relay/${roomId}?mode=host`;
	const receivedByAuthority = new Promise(resolve => authority.once('connection', (socket, request) => socket.once('message', raw => resolve({ socket, hello: JSON.parse(raw.toString()) }))));
	const tunnel = new RoomServiceTunnel(relayUrl, accountKey, internalInvite, `ws://127.0.0.1:${localPort}`, () => undefined, cert);
	t.after(async () => {
		tunnel.close(); relay.close();
		await Promise.all([new Promise(resolve => authority.close(resolve)), new Promise(resolve => service.close(resolve))]);
		fs.rmSync(directory, { recursive: true, force: true });
	});
	await tunnel.connect();
	const guest = new WebSocket(`wss://localhost:${service.address().port}/v1/relay/${roomId}?mode=participant`, { ca: cert, perMessageDeflate: false });
	await once(guest, 'open');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: inviteToken, participantId: randomUUID(), name: 'Bridge guest', role: 'human' }));
	const connected = await Promise.race([receivedByAuthority, new Promise((_, reject) => setTimeout(() => reject(new Error('Local room authority did not receive the relayed hello')), 2000))]);
	assert.equal(connected.hello.secret, internalInvite);
	assert.equal(connected.hello.name, 'Bridge guest');
	const messageFromHost = once(guest, 'message');
	connected.socket.send(JSON.stringify({ version: 16, type: 'notice', text: 'relayed from host' }));
	assert.equal(JSON.parse((await messageFromHost)[0].toString()).text, 'relayed from host');
	const messageAtAuthority = new Promise(resolve => connected.socket.once('message', raw => resolve(JSON.parse(raw.toString()))));
	guest.send(JSON.stringify({ version: 16, type: 'post', text: 'relayed from guest' }));
	const forwarded = await messageAtAuthority;
	assert.equal(forwarded.text, 'relayed from guest');
	guest.terminate();
});
