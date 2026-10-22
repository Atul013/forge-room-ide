'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const { JsonRoomStore, authenticateRoomInvite, createRoomControlPlane } = require('../src/controlPlane');

async function fixture(t, options = {}) {
	const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'forge-room-service-'));
	const filePath = path.join(directory, 'state.json');
	const pfxPath = path.join(directory, 'test-only.pfx');
	const pfx = options.pfx || fs.readFileSync(path.join(__dirname, 'fixtures', 'server.pfx'));
	const cert = options.cert || fs.readFileSync(path.join(__dirname, 'fixtures', 'server-cert.pem'), 'utf8');
	fs.writeFileSync(pfxPath, pfx, { mode: 0o600 });
	const store = new JsonRoomStore(filePath);
	const handle = createRoomControlPlane({ store, adminKey: 'a'.repeat(48), ...(options.controlPlane || {}) });
	const server = https.createServer({ pfx, passphrase: 'test-only-not-for-deployment', minVersion: 'TLSv1.2' }, (request, response) => {
		void handle(request, response).catch(() => { if (!response.headersSent) { response.writeHead(500, { 'content-type': 'application/json' }); } response.end('{"error":"test_server_failure"}'); });
	});
	await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
	t.after(async () => { await new Promise(resolve => server.close(resolve)); fs.rmSync(directory, { recursive: true, force: true }); });
	return { store, filePath, server, base: `https://localhost:${server.address().port}`, cert };
}

async function request(base, cert, route, { method = 'GET', token, body } = {}) {
	return new Promise((resolve, reject) => {
		const data = body === undefined ? undefined : Buffer.from(JSON.stringify(body));
		const req = https.request(`${base}${route}`, { method, ca: cert, headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...(data ? { 'content-type': 'application/json', 'content-length': data.length } : {}) } }, response => {
			const chunks = [];
			response.on('data', chunk => chunks.push(chunk));
			response.on('end', () => resolve({ status: response.statusCode, headers: response.headers, body: JSON.parse(Buffer.concat(chunks).toString('utf8')) }));
		});
		req.on('error', reject);
		if (data) { req.write(data); }
		req.end();
	});
}

test('account admin provisions users; accounts create durable owned rooms and expiring invites', async t => {
	const f = await fixture(t);
	const admin = 'a'.repeat(48);
	assert.equal((await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', body: { name: 'Alice' } })).status, 401);
	const created = await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name: 'Alice' } });
	assert.equal(created.status, 201);
	assert.match(created.body.accountKey, /^fr_account_[A-Za-z0-9_-]{43}$/);
	const room = await request(f.base, f.cert, '/v1/rooms', { method: 'POST', token: created.body.accountKey, body: { name: 'Project Alpha' } });
	assert.equal(room.status, 201);
	const invite = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { method: 'POST', token: created.body.accountKey, body: { label: 'Pairing', expiresInSeconds: 3600 } });
	assert.equal(invite.status, 201);
	assert.match(invite.body.inviteToken, /^fr_invite_[A-Za-z0-9_-]{43}$/);
	assert.equal(JSON.stringify(f.store.state).includes(invite.body.inviteToken), false);
	const stored = JSON.parse(fs.readFileSync(f.filePath, 'utf8'));
	assert.equal(stored.rooms[room.body.room.id].name, 'Project Alpha');
	assert.equal(Object.values(stored.invites)[0].tokenHash.length, 64);
	const restarted = new JsonRoomStore(f.filePath);
	assert.equal(restarted.state.rooms[room.body.room.id].ownerAccountId, created.body.account.id);
	assert.equal(authenticateRoomInvite(restarted, room.body.room.id, invite.body.inviteToken).inviteId, invite.body.invite.id);
	assert.equal(authenticateRoomInvite(restarted, room.body.room.id, 'fr_invite_' + 'x'.repeat(43)), undefined);
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: created.body.accountKey })).body.rooms.length, 1);
	const audit = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/audit`, { token: created.body.accountKey });
	assert.equal(audit.status, 200);
	assert.ok(audit.body.events.some(event => event.action === 'room.created'));
	assert.equal(JSON.stringify(audit.body).includes(invite.body.inviteToken), false);
	assert.equal(JSON.stringify(audit.body).includes(created.body.accountKey), false);
});

test('room ownership prevents cross-account reads, revocation is durable, and disabled accounts require admin recovery', async t => {
	const f = await fixture(t);
	const admin = 'a'.repeat(48);
	const createAccount = async name => (await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name } })).body;
	const alice = await createAccount('Alice');
	const bob = await createAccount('Bob');
	const room = await request(f.base, f.cert, '/v1/rooms', { method: 'POST', token: alice.accountKey, body: { name: 'Private room' } });
	assert.equal((await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { method: 'POST', token: bob.accountKey, body: {} })).status, 404);
	const invite = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { method: 'POST', token: alice.accountKey, body: {} });
	const activeInvite = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { method: 'POST', token: alice.accountKey, body: { label: 'Disabled with account' } });
	const revoked = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites/${invite.body.invite.id}/revoke`, { method: 'POST', token: alice.accountKey, body: {} });
	assert.equal(revoked.body.invite.status, 'revoked');
	assert.equal(authenticateRoomInvite(f.store, room.body.room.id, invite.body.inviteToken), undefined);
	assert.equal((await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { token: alice.accountKey })).body.invites[0].status, 'revoked');
	const disabled = await request(f.base, f.cert, `/v1/admin/accounts/${alice.account.id}/disable`, { method: 'POST', token: admin, body: {} });
	assert.equal(disabled.body.account.status, 'disabled');
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: alice.accountKey })).status, 401);
	assert.equal(authenticateRoomInvite(f.store, room.body.room.id, activeInvite.body.inviteToken), undefined);
	const recovery = await request(f.base, f.cert, `/v1/admin/accounts/${alice.account.id}/rotate-key`, { method: 'POST', token: admin, body: {} });
	assert.equal(recovery.status, 200);
	assert.equal(recovery.body.account.status, 'active');
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: alice.accountKey })).status, 401);
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: recovery.body.accountKey })).status, 200);
	assert.equal(authenticateRoomInvite(f.store, room.body.room.id, activeInvite.body.inviteToken), undefined, 'recovery revokes the disabled account invites instead of restoring them');
});

test('administrator recovers an active account key once, invalidates the lost key, and audits the recovery', async t => {
	const rotatedAccounts = [];
	const f = await fixture(t, { controlPlane: { onAccountKeyRotated: accountId => rotatedAccounts.push(accountId) } });
	const admin = 'a'.repeat(48);
	const created = (await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name: 'Recovered owner' } })).body;
	const room = (await request(f.base, f.cert, '/v1/rooms', { method: 'POST', token: created.accountKey, body: { name: 'Recovery keeps ownership' } })).body.room;
	const route = `/v1/admin/accounts/${created.account.id}/rotate-key`;
	assert.equal((await request(f.base, f.cert, route, { method: 'POST', token: created.accountKey, body: {} })).status, 401);
	const recovered = await request(f.base, f.cert, route, { method: 'POST', token: admin, body: {} });
	assert.equal(recovered.status, 200);
	assert.equal(recovered.body.account.id, created.account.id);
	assert.match(recovered.body.accountKey, /^fr_account_[A-Za-z0-9_-]{43}$/);
	assert.notEqual(recovered.body.accountKey, created.accountKey);
	assert.deepEqual(rotatedAccounts, [created.account.id]);
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: created.accountKey })).status, 401);
	assert.deepEqual((await request(f.base, f.cert, '/v1/rooms', { token: recovered.body.accountKey })).body.rooms.map(item => item.id), [room.id]);
	const auditEvent = f.store.state.audit.find(event => event.action === 'account.key_recovered');
	assert.deepEqual({ actor: auditEvent.actor, resource: auditEvent.resource }, { actor: 'admin', resource: created.account.id });
	assert.equal(JSON.stringify(f.store.state.audit).includes(recovered.body.accountKey), false);
});

test('account owners can rotate their key; the old credential is invalidated and rotation is audited without secrets', async t => {
	let rotatedAccount;
	const f = await fixture(t, { controlPlane: { onAccountKeyRotated: accountId => { rotatedAccount = accountId; } } });
	const admin = 'a'.repeat(48);
	const created = (await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name: 'Key owner' } })).body;
	const room = (await request(f.base, f.cert, '/v1/rooms', { method: 'POST', token: created.accountKey, body: { name: 'Preserved room' } })).body.room;
	const invite = (await request(f.base, f.cert, `/v1/rooms/${room.id}/invites`, { method: 'POST', token: created.accountKey, body: { label: 'Included in audit' } })).body;
	const otherAccount = (await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name: 'Another owner' } })).body;
	const rotated = await request(f.base, f.cert, '/v1/account/rotate-key', { method: 'POST', token: created.accountKey, body: {} });
	assert.equal(rotated.status, 200);
	assert.equal(rotated.body.account.id, created.account.id);
	assert.match(rotated.body.accountKey, /^fr_account_[A-Za-z0-9_-]{43}$/);
	assert.notEqual(rotated.body.accountKey, created.accountKey);
	assert.equal(rotatedAccount, created.account.id);
	assert.equal((await request(f.base, f.cert, '/v1/rooms', { token: created.accountKey })).status, 401);
	assert.deepEqual((await request(f.base, f.cert, '/v1/rooms', { token: rotated.body.accountKey })).body.rooms.map(item => item.id), [room.id]);
	assert.equal((await request(f.base, f.cert, '/v1/account/audit?limit=2')).status, 401);
	const auditResult = await request(f.base, f.cert, '/v1/account/audit?limit=2', { token: rotated.body.accountKey });
	assert.equal(auditResult.status, 200);
	const audit = auditResult.body.events;
	assert.equal(audit.length, 2);
	assert.equal(auditResult.body.nextCursor, audit[0].id);
	assert.ok(audit.some(event => event.action === 'account.key_rotated' && event.actor === created.account.id));
	const olderPage = await request(f.base, f.cert, `/v1/account/audit?limit=2&cursor=${auditResult.body.nextCursor}`, { token: rotated.body.accountKey });
	assert.equal(olderPage.status, 200);
	assert.equal(olderPage.body.events.length, 2);
	assert.equal(olderPage.body.nextCursor, null);
	const allAudit = [...olderPage.body.events, ...audit];
	const ownResources = new Set([created.account.id, room.id, invite.invite.id]);
	assert.ok(allAudit.every(event => ownResources.has(event.resource)), 'account audit returns only this account scope');
	assert.equal(allAudit.some(event => event.resource === otherAccount.account.id), false);
	assert.equal(JSON.stringify(audit).includes(rotated.body.accountKey), false);
	assert.equal((await request(f.base, f.cert, '/v1/account/audit?limit=201', { token: rotated.body.accountKey })).status, 400);
	assert.equal((await request(f.base, f.cert, '/v1/account/audit?cursor=missing', { token: rotated.body.accountKey })).body.error, 'audit_cursor_expired');
	const roomAudit = await request(f.base, f.cert, `/v1/rooms/${room.id}/audit?limit=1`, { token: rotated.body.accountKey });
	assert.equal(roomAudit.status, 200);
	assert.equal(roomAudit.body.events.length, 1);
	assert.ok(roomAudit.body.nextCursor);
	assert.equal(JSON.parse(fs.readFileSync(f.filePath, 'utf8')).accounts[created.account.id].keyHash.length, 64);
});

test('invite expiry is checked against service time and account disable invalidates outstanding invitations', async t => {
	let now = Date.parse('2026-10-21T00:00:00.000Z');
	const f = await fixture(t, { controlPlane: { clock: () => now } });
	const admin = 'a'.repeat(48);
	const created = await request(f.base, f.cert, '/v1/admin/accounts', { method: 'POST', token: admin, body: { name: 'Short lived' } });
	const room = await request(f.base, f.cert, '/v1/rooms', { method: 'POST', token: created.body.accountKey, body: { name: 'Timed room' } });
	const invite = await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { method: 'POST', token: created.body.accountKey, body: { expiresInSeconds: 60 } });
	assert.ok(authenticateRoomInvite(f.store, room.body.room.id, invite.body.inviteToken, now));
	now += 60_001;
	assert.equal(authenticateRoomInvite(f.store, room.body.room.id, invite.body.inviteToken, now), undefined);
	assert.equal((await request(f.base, f.cert, `/v1/rooms/${room.body.room.id}/invites`, { token: created.body.accountKey })).body.invites[0].status, 'expired');
});

test('HTTPS requires a trusted certificate and unauthenticated API access is denied', async t => {
	const f = await fixture(t);
	const health = await request(f.base, f.cert, '/health');
	assert.equal(health.status, 200);
	assert.equal(health.headers['cache-control'], 'no-store');
	assert.equal((await request(f.base, f.cert, '/v1/rooms')).status, 401);
	assert.throws(() => createRoomControlPlane({ store: f.store, adminKey: 'short' }), /32-character/);
});
