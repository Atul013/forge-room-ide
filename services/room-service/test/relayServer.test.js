'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const { once } = require('node:events');
const { randomUUID } = require('node:crypto');
const { WebSocket } = require('ws');
const { JsonRoomStore, tokenHash } = require('../src/controlPlane');
const { createRoomRelay } = require('../src/relayServer');

async function fixture(t, options = {}) {
	const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-relay-'));
	const store = new JsonRoomStore(path.join(directory, 'state.json'));
	const accountId = randomUUID();
	const roomId = randomUUID();
	const inviteId = randomUUID();
	const accountKey = `fr_account_${'A'.repeat(43)}`;
	const inviteToken = `fr_invite_${'B'.repeat(43)}`;
	const createdAt = new Date().toISOString();
	store.commit(state => {
		state.accounts[accountId] = { id: accountId, name: 'Owner', status: 'active', keyHash: tokenHash(accountKey), createdAt };
		state.rooms[roomId] = { id: roomId, ownerAccountId: accountId, name: 'Relay test room', status: 'active', createdAt };
		state.invites[inviteId] = { id: inviteId, roomId, createdBy: accountId, label: 'Test invite', tokenHash: tokenHash(inviteToken), createdAt, expiresAt: new Date(Date.now() + 60_000).toISOString(), revokedAt: null };
	});
	const pfx = fs.readFileSync(path.join(__dirname, 'fixtures', 'server.pfx'));
	const cert = fs.readFileSync(path.join(__dirname, 'fixtures', 'server-cert.pem'));
	const server = https.createServer({ pfx, passphrase: 'test-only-not-for-deployment', minVersion: 'TLSv1.2' }, (_request, response) => { response.writeHead(404); response.end(); });
	const relay = createRoomRelay({ httpsServer: server, store, ...(options.relay || {}) });
	await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
	t.after(async () => { relay.close(); await new Promise(resolve => server.close(resolve)); fs.rmSync(directory, { recursive: true, force: true }); });
	return { store, relay, accountId, roomId, inviteId, accountKey, inviteToken, url: `wss://localhost:${server.address().port}/v1/relay/${roomId}`, cert };
}

function connect(url, cert, mode, accountKey) {
	return new WebSocket(`${url}?mode=${mode}`, { ca: cert, ...(accountKey ? { headers: { authorization: `Bearer ${accountKey}` } } : {}) });
}

function nextJson(socket, predicate = () => true) {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => { cleanup(); reject(new Error('Timed out waiting for relay message')); }, 2000);
		const onMessage = raw => { let value; try { value = JSON.parse(raw.toString()); } catch { return; } if (predicate(value)) { cleanup(); resolve(value); } };
		const onError = error => { cleanup(); reject(error); };
		const cleanup = () => { clearTimeout(timeout); socket.off('message', onMessage); socket.off('error', onError); };
		socket.on('message', onMessage);
		socket.on('error', onError);
	});
}

test('TLS relay authenticates the room owner and invite, then forwards messages in both directions', async t => {
	const f = await fixture(t);
	const badHost = connect(f.url, f.cert, 'host', `fr_account_${'Z'.repeat(43)}`);
	await assert.rejects(once(badHost, 'open'), /Unexpected server response: 401/);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const hostJoin = nextJson(host, item => item.type === 'clientConnect');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: f.inviteToken, participantId: randomUUID(), name: 'Guest', role: 'human' }));
	const joined = await hostJoin;
	assert.equal(joined.hello.name, 'Guest');
	assert.ok(f.relay.rooms.get(f.roomId).clients.has(joined.clientId));
	const forwardedToHost = nextJson(host, item => item.type === 'clientMessage');
	guest.send(JSON.stringify({ version: 16, type: 'post', text: 'hello room' }));
	assert.equal(JSON.parse((await forwardedToHost).data).text, 'hello room');
	const forwardedToGuest = nextJson(guest, item => item.type === 'history');
	host.send(JSON.stringify({ relayVersion: 1, type: 'clientMessage', clientId: joined.clientId, data: JSON.stringify({ version: 16, type: 'history', events: [] }) }));
	assert.deepEqual(await forwardedToGuest, { version: 16, type: 'history', events: [] });
	const closed = once(guest, 'close');
	f.relay.revokeInvite(f.inviteId);
	const [code] = await closed;
	assert.equal(code, 4003);
	assert.equal(f.relay.rooms.get(f.roomId).clients.size, 0);
	host.terminate();
});

test('invalid invitation closes before creating a room peer and room accounts can be disabled', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const closed = once(guest, 'close');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: `fr_invite_${'Z'.repeat(43)}`, participantId: randomUUID(), name: 'Mallory', role: 'human' }));
	const [code] = await closed;
	assert.equal(code, 1008);
	assert.equal(f.relay.rooms.get(f.roomId).clients.size, 0);
	const roomClosed = once(host, 'close');
	f.relay.disableAccount(f.accountId);
	assert.equal((await roomClosed)[0], 4003);
});

test('rotating an account key disconnects its hosted rooms and guests', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const hostJoin = nextJson(host, item => item.type === 'clientConnect');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: f.inviteToken, participantId: randomUUID(), name: 'Guest', role: 'human' }));
	const joined = await hostJoin;
	assert.ok(f.relay.rooms.get(f.roomId).clients.has(joined.clientId));
	const hostClosed = once(host, 'close');
	const guestClosed = once(guest, 'close');
	f.relay.invalidateAccountSessions(f.accountId);
	assert.equal((await hostClosed)[0], 1012);
	assert.equal((await guestClosed)[0], 1012);
	assert.equal(f.relay.rooms.has(f.roomId), false);
});

test('malformed and oversized guest frames are rejected before room messages are relayed', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const malformed = connect(f.url, f.cert, 'participant');
	await once(malformed, 'open');
	const malformedClosed = once(malformed, 'close');
	malformed.send('{');
	assert.equal((await malformedClosed)[0], 1008);
	assert.equal(f.relay.rooms.get(f.roomId).clients.size, 0);

	const oversized = connect(f.url, f.cert, 'participant');
	await once(oversized, 'open');
	const oversizedClosed = once(oversized, 'close');
	oversized.send(Buffer.alloc(2 * 1024 * 1024 + 1));
	assert.equal((await oversizedClosed)[0], 1009);
	assert.equal(f.relay.rooms.get(f.roomId).clients.size, 0);
	host.terminate();
});

test('guest message floods are closed while the room remains available', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const joinedAtHost = nextJson(host, item => item.type === 'clientConnect');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: f.inviteToken, participantId: randomUUID(), name: 'Guest', role: 'human' }));
	await joinedAtHost;
	const guestClosed = once(guest, 'close');
	const post = JSON.stringify({ version: 16, type: 'post', text: 'x' });
	for (let index = 0; index < 1000; index++) { guest.send(post); }
	const [code] = await guestClosed;
	assert.equal(code, 1008);
	assert.equal(f.relay.rooms.get(f.roomId).clients.size, 0);
	assert.ok(f.relay.rooms.has(f.roomId));
	assert.equal(host.readyState, WebSocket.OPEN);
	host.terminate();
});

test('host message floods close the room and its guests', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const joinedAtHost = nextJson(host, item => item.type === 'clientConnect');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: f.inviteToken, participantId: randomUUID(), name: 'Guest', role: 'human' }));
	const { clientId } = await joinedAtHost;
	const hostClosed = once(host, 'close');
	const guestClosed = once(guest, 'close');
	const frame = JSON.stringify({ relayVersion: 1, type: 'clientMessage', clientId, data: JSON.stringify({ version: 16, type: 'heartbeat' }) });
	for (let index = 0; index < 1001; index++) { host.send(frame); }
	assert.equal((await hostClosed)[0], 1013);
	assert.equal((await guestClosed)[0], 1013);
	assert.equal(f.relay.rooms.has(f.roomId), false);
});

test('relay upgrades are rate limited per remote address', async t => {
	const f = await fixture(t, { relay: { maxConnectionAttemptsPerMinute: 2 } });
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const flood = connect(f.url, f.cert, 'participant');
	await assert.rejects(once(flood, 'open'), /Unexpected server response: 429/);
	host.terminate();
	guest.terminate();
});

test('host network loss closes its guests and removes the in-memory relay room', async t => {
	const f = await fixture(t);
	const host = connect(f.url, f.cert, 'host', f.accountKey);
	await once(host, 'open');
	const guest = connect(f.url, f.cert, 'participant');
	await once(guest, 'open');
	const joinedAtHost = nextJson(host, item => item.type === 'clientConnect');
	guest.send(JSON.stringify({ version: 16, type: 'hello', secret: f.inviteToken, participantId: randomUUID(), name: 'Guest', role: 'human' }));
	const { clientId } = await joinedAtHost;
	assert.ok(f.relay.rooms.get(f.roomId).clients.has(clientId));
	const guestClosed = once(guest, 'close');
	host.terminate();
	assert.equal((await guestClosed)[0], 1012);
	assert.equal(f.relay.rooms.has(f.roomId), false);
});
