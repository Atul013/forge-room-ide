'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { randomBytes, randomUUID, createHash, timingSafeEqual } = require('node:crypto');

const maxBodyBytes = 16 * 1024;
const maxAuditEntries = 10_000;
const defaultInviteLifetimeSeconds = 12 * 60 * 60;
const maxInviteLifetimeSeconds = 30 * 24 * 60 * 60;
const maxAuditPageSize = 200;

function tokenHash(value) { return createHash('sha256').update(value, 'utf8').digest('hex'); }
function safeEqual(left, right) {
	const a = Buffer.from(left, 'hex');
	const b = Buffer.from(right, 'hex');
	return a.length === b.length && a.length > 0 && timingSafeEqual(a, b);
}
function bearer(request) {
	const header = request.headers.authorization;
	const match = typeof header === 'string' && /^Bearer ([A-Za-z0-9_-]{40,160})$/.exec(header);
	return match?.[1];
}
function validLabel(value, max = 80) {
	return typeof value === 'string' && value.trim().length > 0 && value.trim().length <= max && !/[\u0000-\u001f\u007f]/.test(value);
}
function pageAuditEvents(events, params) {
	if ([...params.keys()].some(key => !['limit', 'cursor'].includes(key)) || params.getAll('limit').length > 1 || params.getAll('cursor').length > 1) { return { error: 'invalid_audit_query' }; }
	const rawLimit = params.get('limit');
	const limit = rawLimit === null ? 100 : /^[1-9][0-9]{0,2}$/.test(rawLimit) ? Number(rawLimit) : 0;
	if (limit < 1 || limit > maxAuditPageSize) { return { error: 'invalid_audit_limit' }; }
	const cursor = params.get('cursor');
	let end = events.length;
	if (cursor !== null) {
		if (!/^[A-Za-z0-9_-]{1,160}$/.test(cursor)) { return { error: 'invalid_audit_cursor' }; }
		const cursorIndex = events.findIndex(event => event.id === cursor);
		if (cursorIndex < 0) { return { error: 'audit_cursor_expired' }; }
		end = cursorIndex;
	}
	const start = Math.max(0, end - limit);
	return { events: events.slice(start, end), nextCursor: start > 0 ? events[start].id : null };
}
function freshState() { return { version: 1, accounts: {}, rooms: {}, invites: {}, audit: [] }; }
function validState(value) {
	if (!value || value.version !== 1 || !value.accounts || typeof value.accounts !== 'object' || !value.rooms || typeof value.rooms !== 'object' || !value.invites || typeof value.invites !== 'object' || !Array.isArray(value.audit) || value.audit.length > maxAuditEntries) { throw new Error('Invalid service database schema.'); }
	return value;
}

/** Resolve a one-time-issued invite without exposing its stored hash. */
function authenticateRoomInvite(store, roomId, token, now = Date.now()) {
	if (!store || typeof roomId !== 'string' || typeof token !== 'string' || !/^fr_invite_[A-Za-z0-9_-]{43}$/.test(token)) { return undefined; }
	const hash = tokenHash(token);
	const invite = Object.values(store.state.invites).find(item => item.roomId === roomId && safeEqual(item.tokenHash, hash));
	if (!invite || invite.revokedAt || Date.parse(invite.expiresAt) <= now) { return undefined; }
	const room = store.state.rooms[roomId];
	const account = room && store.state.accounts[room.ownerAccountId];
	if (!room || room.status !== 'active' || !account || account.status !== 'active') { return undefined; }
	return { roomId, inviteId: invite.id, accountId: account.id };
}

function authenticateAccountKey(store, token) {
	if (!store || typeof token !== 'string' || !/^fr_account_[A-Za-z0-9_-]{43}$/.test(token)) { return undefined; }
	const hash = tokenHash(token);
	return Object.values(store.state.accounts).find(account => account.status === 'active' && safeEqual(account.keyHash, hash));
}

class JsonRoomStore {
	constructor(filePath) { this.filePath = path.resolve(filePath); this.state = this.load(); }
	load() {
		try { return validState(JSON.parse(fs.readFileSync(this.filePath, 'utf8'))); }
		catch (error) { if (error.code === 'ENOENT') { return freshState(); } throw new Error(`Could not load room service database: ${error.message}`); }
	}
	commit(mutator) {
		const next = structuredClone(this.state);
		const result = mutator(next);
		const directory = path.dirname(this.filePath);
		fs.mkdirSync(directory, { recursive: true, mode: 0o700 });
		const temporary = `${this.filePath}.${process.pid}.${randomUUID()}.tmp`;
		try {
			fs.writeFileSync(temporary, `${JSON.stringify(next)}\n`, { encoding: 'utf8', mode: 0o600, flag: 'wx' });
			const descriptor = fs.openSync(temporary, 'r+');
			try { fs.fsyncSync(descriptor); } finally { fs.closeSync(descriptor); }
			fs.renameSync(temporary, this.filePath);
			if (process.platform !== 'win32') { const directoryDescriptor = fs.openSync(directory, 'r'); try { fs.fsyncSync(directoryDescriptor); } finally { fs.closeSync(directoryDescriptor); } }
			this.state = next;
			return result;
		} finally { try { fs.unlinkSync(temporary); } catch { /* renamed or already removed */ } }
	}
}

function createRoomControlPlane({ store, adminKey, clock = () => Date.now(), id = randomUUID, randomToken = () => randomBytes(32).toString('base64url'), maxRequestsPerMinute = 120, onInviteRevoked = () => {}, onAccountDisabled = () => {}, onAccountKeyRotated = () => {} }) {
	if (!store || typeof store.commit !== 'function' || typeof adminKey !== 'string' || adminKey.length < 32) { throw new Error('A durable store and at least 32-character admin key are required.'); }
	const adminHash = tokenHash(adminKey);
	const requests = new Map();
	function audit(state, action, actor, resource) {
		state.audit.push({ id: id(), at: new Date(clock()).toISOString(), action, actor, resource });
		if (state.audit.length > maxAuditEntries) { state.audit.splice(0, state.audit.length - maxAuditEntries); }
	}
	const accountFor = token => authenticateAccountKey(store, token);
	return async function handle(request, response) {
		const now = clock();
		const address = request.socket.remoteAddress || 'unknown';
		const bucket = requests.get(address) || { start: now, count: 0 };
		if (now - bucket.start >= 60_000) { bucket.start = now; bucket.count = 0; }
		bucket.count++;
		if (requests.size > 4096) { for (const [key, value] of requests) { if (now - value.start >= 60_000) { requests.delete(key); } } }
		if (!requests.has(address) && requests.size >= 4096) { return reply(response, 429, { error: 'rate_limited' }); }
		requests.set(address, bucket);
		if (bucket.count > maxRequestsPerMinute) { return reply(response, 429, { error: 'rate_limited' }); }
		if (request.method === 'GET' && request.url === '/health') { return reply(response, 200, { service: 'forge-room-room-service', status: 'ok' }); }
		let body;
		try { body = await readJson(request); }
		catch (error) { return reply(response, error.status || 400, { error: error.code || 'invalid_json' }); }
		const token = bearer(request);
		const isAdmin = !!token && safeEqual(adminHash, tokenHash(token));
		const account = accountFor(token);
		const method = request.method;
		const url = request.url;
		const parsedUrl = new URL(url, 'https://forge-room.invalid');

		if (method === 'POST' && url === '/v1/admin/accounts') {
			if (!isAdmin) { return reply(response, 401, { error: 'admin_auth_required' }); }
			if (!validLabel(body?.name, 80)) { return reply(response, 400, { error: 'invalid_account_name' }); }
			const accountId = id();
			const accountKey = `fr_account_${randomToken()}`;
			store.commit(state => { state.accounts[accountId] = { id: accountId, name: body.name.trim(), keyHash: tokenHash(accountKey), status: 'active', createdAt: new Date(now).toISOString() }; audit(state, 'account.created', 'admin', accountId); });
			return reply(response, 201, { account: { id: accountId, name: body.name.trim(), status: 'active' }, accountKey });
		}
		if (method === 'GET' && url === '/v1/admin/accounts') {
			if (!isAdmin) { return reply(response, 401, { error: 'admin_auth_required' }); }
			return reply(response, 200, { accounts: Object.values(store.state.accounts).map(({ id: accountId, name, status, createdAt }) => ({ id: accountId, name, status, createdAt })) });
		}
		const adminRotateMatch = method === 'POST' && /^\/v1\/admin\/accounts\/([0-9a-f-]{36})\/rotate-key$/i.exec(url);
		if (adminRotateMatch) {
			if (!isAdmin) { return reply(response, 401, { error: 'admin_auth_required' }); }
			const target = store.state.accounts[adminRotateMatch[1]];
			if (!target) { return reply(response, 404, { error: 'account_not_found' }); }
			if (!['active', 'disabled'].includes(target.status)) { return reply(response, 409, { error: 'account_not_recoverable' }); }
			const accountKey = `fr_account_${randomToken()}`;
			const revokedInviteIds = [];
			const restoringDisabledAccount = target.status === 'disabled';
			const recoveredAt = new Date(now).toISOString();
			store.commit(state => {
				state.accounts[target.id].keyHash = tokenHash(accountKey);
				state.accounts[target.id].status = 'active';
				if (restoringDisabledAccount) {
					const ownedRoomIds = new Set(Object.values(state.rooms).filter(room => room.ownerAccountId === target.id).map(room => room.id));
					for (const invite of Object.values(state.invites)) {
						if (ownedRoomIds.has(invite.roomId) && !invite.revokedAt) {
							invite.revokedAt = recoveredAt;
							revokedInviteIds.push(invite.id);
							audit(state, 'invite.revoked', 'admin', invite.id);
						}
					}
				}
				audit(state, 'account.key_recovered', 'admin', target.id);
			});
			for (const inviteId of revokedInviteIds) { onInviteRevoked(inviteId); }
			onAccountKeyRotated(target.id);
			return reply(response, 200, { account: { id: target.id, name: target.name, status: 'active' }, accountKey });
		}
		if (method === 'POST' && url === '/v1/account/rotate-key') {
			if (!account) { return reply(response, 401, { error: 'account_auth_required' }); }
			const accountKey = `fr_account_${randomToken()}`;
			store.commit(state => {
				state.accounts[account.id].keyHash = tokenHash(accountKey);
				audit(state, 'account.key_rotated', account.id, account.id);
			});
			onAccountKeyRotated(account.id);
			return reply(response, 200, { account: { id: account.id, status: 'active' }, accountKey });
		}
		if (method === 'GET' && parsedUrl.pathname === '/v1/account/audit') {
			if (!account) { return reply(response, 401, { error: 'account_auth_required' }); }
			const roomIds = new Set(Object.values(store.state.rooms).filter(room => room.ownerAccountId === account.id).map(room => room.id));
			const inviteIds = new Set(Object.values(store.state.invites).filter(invite => roomIds.has(invite.roomId)).map(invite => invite.id));
			const events = store.state.audit.filter(event => event.resource === account.id || roomIds.has(event.resource) || inviteIds.has(event.resource));
			const page = pageAuditEvents(events, parsedUrl.searchParams);
			return page.error ? reply(response, 400, { error: page.error }) : reply(response, 200, page);
		}
		const disableMatch = method === 'POST' && /^\/v1\/admin\/accounts\/([0-9a-f-]{36})\/disable$/i.exec(url);
		if (disableMatch) {
			if (!isAdmin) { return reply(response, 401, { error: 'admin_auth_required' }); }
			const target = store.state.accounts[disableMatch[1]];
			if (!target) { return reply(response, 404, { error: 'account_not_found' }); }
			store.commit(state => { state.accounts[target.id].status = 'disabled'; audit(state, 'account.disabled', 'admin', target.id); });
			onAccountDisabled(target.id);
			return reply(response, 200, { account: { id: target.id, status: 'disabled' } });
		}
		if (!account) { return reply(response, 401, { error: 'account_auth_required' }); }
		if (method === 'POST' && url === '/v1/rooms') {
			if (!validLabel(body?.name, 120)) { return reply(response, 400, { error: 'invalid_room_name' }); }
			const roomId = id();
			const name = body.name.trim();
			store.commit(state => { state.rooms[roomId] = { id: roomId, ownerAccountId: account.id, name, createdAt: new Date(now).toISOString(), status: 'active' }; audit(state, 'room.created', account.id, roomId); });
			return reply(response, 201, { room: store.state.rooms[roomId] });
		}
		if (method === 'GET' && url === '/v1/rooms') {
			const rooms = Object.values(store.state.rooms).filter(room => room.ownerAccountId === account.id);
			return reply(response, 200, { rooms });
		}
		const inviteMatch = method === 'POST' && /^\/v1\/rooms\/([0-9a-f-]{36})\/invites$/.exec(url);
		if (inviteMatch) {
			const room = store.state.rooms[inviteMatch[1]];
			if (!room || room.ownerAccountId !== account.id) { return reply(response, 404, { error: 'room_not_found' }); }
			const lifetime = body?.expiresInSeconds === undefined ? defaultInviteLifetimeSeconds : body.expiresInSeconds;
			if (!Number.isInteger(lifetime) || lifetime < 60 || lifetime > maxInviteLifetimeSeconds || body?.label !== undefined && !validLabel(body.label, 80)) { return reply(response, 400, { error: 'invalid_invite' }); }
			const inviteId = id();
			const inviteToken = `fr_invite_${randomToken()}`;
			const invite = { id: inviteId, roomId: room.id, createdBy: account.id, label: body.label?.trim() || 'Room invite', tokenHash: tokenHash(inviteToken), createdAt: new Date(now).toISOString(), expiresAt: new Date(now + lifetime * 1000).toISOString(), revokedAt: null };
			store.commit(state => { state.invites[inviteId] = invite; audit(state, 'invite.created', account.id, inviteId); });
			return reply(response, 201, { invite: { id: inviteId, roomId: room.id, label: invite.label, createdAt: invite.createdAt, expiresAt: invite.expiresAt }, inviteToken });
		}
		const revokeMatch = method === 'POST' && /^\/v1\/rooms\/([0-9a-f-]{36})\/invites\/([0-9a-f-]{36})\/revoke$/.exec(url);
		if (revokeMatch) {
			const room = store.state.rooms[revokeMatch[1]];
			const invite = store.state.invites[revokeMatch[2]];
			if (!room || room.ownerAccountId !== account.id || !invite || invite.roomId !== room.id) { return reply(response, 404, { error: 'invite_not_found' }); }
			if (invite.revokedAt) { return reply(response, 200, { invite: { id: invite.id, status: 'revoked' } }); }
			const revokedAt = new Date(now).toISOString();
			store.commit(state => { state.invites[invite.id].revokedAt = revokedAt; audit(state, 'invite.revoked', account.id, invite.id); });
			onInviteRevoked(invite.id);
			return reply(response, 200, { invite: { id: invite.id, status: 'revoked', revokedAt } });
		}
		if (method === 'GET' && /^\/v1\/rooms\/[0-9a-f-]{36}\/invites$/.test(url)) {
			const roomId = url.split('/')[3];
			const room = store.state.rooms[roomId];
			if (!room || room.ownerAccountId !== account.id) { return reply(response, 404, { error: 'room_not_found' }); }
			const invites = Object.values(store.state.invites).filter(invite => invite.roomId === roomId).map(invite => ({ id: invite.id, roomId, label: invite.label, createdAt: invite.createdAt, expiresAt: invite.expiresAt, revokedAt: invite.revokedAt, status: invite.revokedAt ? 'revoked' : Date.parse(invite.expiresAt) <= now ? 'expired' : 'active' }));
			return reply(response, 200, { invites });
		}
		const scopedAuditMatch = method === 'GET' && /^\/v1\/rooms\/([0-9a-f-]{36})\/audit$/i.exec(parsedUrl.pathname);
		if (scopedAuditMatch) {
			const room = store.state.rooms[scopedAuditMatch[1]];
			if (!room || room.ownerAccountId !== account.id) { return reply(response, 404, { error: 'room_not_found' }); }
			const inviteIds = new Set(Object.values(store.state.invites).filter(invite => invite.roomId === room.id).map(invite => invite.id));
			const events = store.state.audit.filter(event => event.resource === room.id || inviteIds.has(event.resource));
			const page = pageAuditEvents(events, parsedUrl.searchParams);
			return page.error ? reply(response, 400, { error: page.error }) : reply(response, 200, page);
		}
		return reply(response, 404, { error: 'not_found' });
	};
}

function readJson(request) {
	return new Promise((resolve, reject) => {
		if (request.method === 'GET' || request.method === 'HEAD') { resolve(undefined); return; }
		let size = 0;
		const chunks = [];
		request.on('data', chunk => { size += chunk.length; if (size > maxBodyBytes) { const error = Object.assign(new Error('too_large'), { status: 413, code: 'body_too_large' }); reject(error); request.destroy(); return; } chunks.push(chunk); });
		request.on('end', () => {
			if (!size) { resolve({}); return; }
			try { const value = JSON.parse(Buffer.concat(chunks).toString('utf8')); if (!value || typeof value !== 'object' || Array.isArray(value)) { throw new Error(); } resolve(value); }
			catch { reject(Object.assign(new Error('invalid_json'), { status: 400, code: 'invalid_json' })); }
		});
		request.on('error', reject);
	});
}
function reply(response, status, value) {
	const body = JSON.stringify(value);
	response.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'content-length': Buffer.byteLength(body), 'cache-control': 'no-store', 'x-content-type-options': 'nosniff' });
	response.end(body);
}

module.exports = { JsonRoomStore, authenticateAccountKey, authenticateRoomInvite, createRoomControlPlane, defaultInviteLifetimeSeconds, maxInviteLifetimeSeconds, tokenHash };
