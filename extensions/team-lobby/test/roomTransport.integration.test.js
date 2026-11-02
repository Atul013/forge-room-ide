const test = require('node:test');
const assert = require('node:assert/strict');
const { once } = require('node:events');
const { randomUUID } = require('node:crypto');
const { WebSocket, WebSocketServer } = require('ws');
const { acceptRoomSocket } = require('../dist/roomTransport');
const { canDeliverMessageEvent, directMessagePeers } = require('../dist/directMessages');
const { taskWriteDenial } = require('../dist/taskAuthorization');
const { acceptInviteIdentity } = require('../dist/inviteIdentity');
const { revokeInvite } = require('../dist/inviteRevocation');
const { validRoomArtifact } = require('../dist/roomArtifacts');
const { prepareEditorLocationUpdate } = require('../dist/sharedLocation');

const protocolVersion = 14;
const socketQueues = new WeakMap();

function waitForMessage(socket, predicate, timeoutMs = 2000) {
	return new Promise((resolve, reject) => {
		const state = socketQueues.get(socket);
		const queuedIndex = state?.queue.findIndex(predicate) ?? -1;
		if (queuedIndex >= 0) { resolve(state.queue.splice(queuedIndex, 1)[0]); return; }
		const timeout = setTimeout(() => { cleanup(); reject(new Error('Timed out waiting for room message')); }, timeoutMs);
		const waiter = { predicate, resolve: message => { cleanup(); resolve(message); } };
		const cleanup = () => { clearTimeout(timeout); const index = state.waiters.indexOf(waiter); if (index >= 0) { state.waiters.splice(index, 1); } };
		state.waiters.push(waiter);
	});
}

function trackSocket(socket) {
	const state = { queue: [], waiters: [] };
	socketQueues.set(socket, state);
	socket.on('message', raw => {
		let message;
		try { message = JSON.parse(raw.toString()); } catch { return; }
		const waiter = state.waiters.find(candidate => candidate.predicate(message));
		if (waiter) { waiter.resolve(message); } else { state.queue.push(message); }
	});
}

function createRoomHarness(server, inviteToken) {
	const peers = new Map();
	const identities = new Map();
	const invites = new Map([[inviteToken, true]]);
	const events = [];
	const tasks = new Map();
	const send = (peer, message) => { if (peer.socket.readyState === WebSocket.OPEN) { peer.socket.send(JSON.stringify({ version: protocolVersion, ...message })); } };
	const broadcastEvent = event => {
		const audited = { ...event, at: event.at || new Date().toISOString() };
		events.push(audited);
		for (const peer of peers.values()) { send(peer, { type: 'event', event: audited }); }
	};
	const broadcastPresence = () => {
		const members = [{ id: 'host-id', name: 'Host', role: 'human' }, ...[...peers.values()].flatMap(({ id, name, role, agent }) => [{ id, name, role }, ...(agent ? [agent] : [])])];
		for (const peer of peers.values()) { send(peer, { type: 'count', count: members.length, members }); }
	};
	server.on('connection', socket => acceptRoomSocket(socket, {
		protocolVersion,
		authenticationTimeoutMs: 1000,
		authenticate(message, clientSocket) {
			if (message.type !== 'hello' || typeof message.secret !== 'string' || !invites.has(message.secret) || typeof message.participantId !== 'string' || !/^[0-9a-f-]{36}$/i.test(message.participantId) || typeof message.name !== 'string' || !['human', 'agent'].includes(message.role)) {
				return { error: 'Invalid or expired room invite' };
			}
			const id = message.participantId;
			const acceptedIdentity = acceptInviteIdentity(identities, id, { name: message.name, role: message.role });
			if (!acceptedIdentity.accepted) { return { error: 'Participant identity cannot change for this invite' }; }
			const previous = peers.get(id);
			if (previous) { previous.socket.close(4001, 'Participant reconnected'); }
			const peer = { id, ...acceptedIdentity.identity, inviteToken: message.secret, socket: clientSocket };
			peers.set(id, peer);
			return {
				peer,
				history: { version: protocolVersion, type: 'history', host: { name: 'Host', role: 'human' }, members: [{ id: 'host-id', name: 'Host', role: 'human' }, ...[...peers.values()].flatMap(({ id, name, role, agent }) => [{ id, name, role }, ...(agent ? [agent] : [])])], events: events.filter(event => canDeliverMessageEvent(event, id)), tasks: [...tasks.values()] }
			};
		},
		isAuthorized: peer => invites.has(peer.inviteToken),
		isCurrent: peer => peers.get(peer.id) === peer,
		onAccepted(peer) {
			broadcastEvent({ id: randomUUID(), kind: 'join', participantId: peer.id, name: peer.name });
			broadcastPresence();
		},
			onMessage(peer, message) {
				if (message.type === 'agentPresence') {
					const presence = message.presence;
					if (presence === null) { peer.agent = undefined; broadcastPresence(); return; }
					const task = tasks.get(presence?.taskId);
					if (!presence || presence.role !== 'agent' || presence.ownerId !== peer.id || !task || task.ownerId !== peer.id || !/^[0-9a-f-]{36}$/i.test(presence.id) || typeof presence.name !== 'string' || presence.name.length > 40 || !['working', 'review'].includes(presence.status)) { send(peer, { type: 'notice', detail: 'Agent presence must be linked to a task assigned to this participant.' }); return; }
					peer.agent = { id: presence.id, name: presence.name, role: 'agent', ownerId: peer.id, status: presence.status, taskId: task.id };
					broadcastPresence();
				} else if (message.type === 'event' && message.kind === 'post' && typeof message.detail === 'string') {
				broadcastEvent({ id: randomUUID(), kind: 'post', participantId: peer.id, name: peer.name, detail: message.detail.slice(0, 500) });
			} else if (message.type === 'event' && message.kind === 'message' && typeof message.recipientId === 'string' && typeof message.detail === 'string') {
				const target = peers.get(message.recipientId);
				if (!target || target.id === peer.id) { send(peer, { type: 'notice', detail: 'That direct-message recipient is not connected.' }); return; }
				const event = { id: randomUUID(), kind: 'message', participantId: peer.id, name: peer.name, role: peer.role, detail: message.detail.slice(0, 500), recipient: target.name, recipientId: target.id, senderId: peer.id };
				events.push(event);
				for (const recipient of directMessagePeers([...peers.values()], target.id, peer.id)) { send(recipient, { type: 'event', event }); }
			} else if (message.type === 'editorLocation') {
				const prepared = prepareEditorLocationUpdate(message, { id: peer.id, name: peer.name, role: peer.role }, peer.lastLocationAt, Date.now());
				if (prepared.cleared && peer.location) {
					peer.location = undefined;
					for (const recipient of peers.values()) { send(recipient, { type: 'editorLocation', active: false, participantId: peer.id }); }
				} else if (prepared.location) {
					peer.lastLocationAt = prepared.lastSentAt;
					peer.location = prepared.location;
					for (const recipient of peers.values()) { send(recipient, { type: 'editorLocation', active: true, location: prepared.location }); }
				}
			} else if (message.type === 'event' && message.kind === 'task' && message.task && typeof message.task === 'object') {
				const next = message.task;
				const current = tasks.get(next.id);
				const connected = new Set(['host-id', ...peers.keys()]);
				const denial = taskWriteDenial(peer.id, 'host-id', current, next, connected);
				if (denial) { send(peer, { type: 'notice', detail: denial }); return; }
				tasks.set(next.id, next);
				broadcastEvent({ id: randomUUID(), kind: 'task', participantId: peer.id, name: peer.name, task: next });
			} else if (message.type === 'event' && message.kind === 'review' && typeof message.taskId === 'string') {
				const linkedTask = tasks.get(message.taskId);
				if (!linkedTask) { send(peer, { type: 'notice', detail: 'The linked review task is no longer available.' }); return; }
				broadcastEvent({ id: randomUUID(), kind: 'review', participantId: peer.id, name: peer.name, detail: String(message.detail).slice(0, 500), taskId: message.taskId });
			} else if (message.type === 'event' && message.kind === 'agent' && typeof message.taskId === 'string') {
				const linkedTask = tasks.get(message.taskId);
				if (!linkedTask || linkedTask.ownerId !== peer.id) { send(peer, { type: 'notice', detail: 'Only the assigned task owner or host can publish task-linked agent activity.' }); return; }
				broadcastEvent({ id: randomUUID(), kind: 'agent', participantId: peer.id, name: peer.name, detail: String(message.detail).slice(0, 500), taskId: message.taskId });
			} else if (message.type === 'event' && validRoomArtifact(message)) {
				const { kind, detail, file, patch, anchor } = message;
				broadcastEvent({ id: randomUUID(), kind, participantId: peer.id, name: peer.name, role: peer.role, detail, file, ...(patch ? { patch } : {}), ...(anchor ? { anchor } : {}) });
			}
		},
		onClosed(peer) {
			peers.delete(peer.id);
			broadcastEvent({ id: randomUUID(), kind: 'leave', participantId: peer.id, name: peer.name });
			broadcastPresence();
		}
	}));
	return {
		peers,
		events,
		addInvite: token => invites.set(token, true),
		revoke(token) {
			const result = revokeInvite(invites, peers, token);
			for (const peer of result.peers) { peer.socket.close(4003, 'Room invite revoked'); }
			if (result.revoked) { broadcastEvent({ id: randomUUID(), kind: 'revoke', participantId: 'host-id', name: 'Host', role: 'human', detail: `revoked an invite and disconnected ${result.peers.length} participant(s)` }); }
			return result;
		}
	};
}

async function startServer(t, inviteToken) {
	const server = new WebSocketServer({ host: '127.0.0.1', port: 0, maxPayload: 8192 });
	await once(server, 'listening');
	t.after(async () => {
		for (const client of server.clients) { client.terminate(); }
		await new Promise(resolve => server.close(resolve));
	});
	const address = server.address();
	return { server, url: `ws://127.0.0.1:${address.port}`, room: createRoomHarness(server, inviteToken) };
}

function connect(url, participantId, name, secret = 'room-secret') {
	const socket = new WebSocket(url);
	trackSocket(socket);
	const opened = once(socket, 'open');
	return opened.then(() => {
		socket.send(JSON.stringify({ version: protocolVersion, type: 'hello', secret, participantId, name, role: 'human' }));
		return socket;
	});
}

test('two LAN clients receive room history, presence, shared events, and reconnect with stable identity', async t => {
	const { server, url, room } = await startServer(t, 'room-secret');
	const firstId = randomUUID();
	const secondId = randomUUID();
	const first = await connect(url, firstId, 'Ada');
	const firstHistory = await waitForMessage(first, message => message.type === 'history');
	assert.deepEqual(firstHistory.members.map(member => member.id), ['host-id', firstId]);

	const firstJoin = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'join' && message.event.participantId === secondId);
	const firstPresence = waitForMessage(first, message => message.type === 'count' && message.members?.some(member => member.id === secondId));
	const second = await connect(url, secondId, 'Lin');
	t.after(() => { first.terminate(); second.terminate(); });
	const secondHistory = await waitForMessage(second, message => message.type === 'history');
	assert.deepEqual(secondHistory.members.map(member => member.id), ['host-id', firstId, secondId]);
	await firstJoin;
	assert.equal((await firstPresence).count, 3);
	const observerId = randomUUID();
	const observer = await connect(url, observerId, 'Sam');
	t.after(() => observer.terminate());
	const observerHistory = await waitForMessage(observer, message => message.type === 'history');
	assert.equal(observerHistory.tasks.length, 0);

	const firstPost = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'post' && message.event.detail === 'ready for review');
	const secondPost = waitForMessage(second, message => message.type === 'event' && message.event?.kind === 'post' && message.event.detail === 'ready for review');
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'post', detail: 'ready for review' }));
	assert.equal((await firstPost).event.participantId, secondId);
	assert.equal((await secondPost).event.name, 'Lin');

	const taskId = randomUUID();
	const task = { id: taskId, title: 'Run the integration checks', owner: 'Lin', ownerId: secondId, status: 'open', updatedAt: new Date().toISOString() };
	const syncedTask = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'task' && message.event.task?.id === taskId);
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'task', detail: 'created a task', task }));
	assert.equal((await syncedTask).event.task.ownerId, secondId);
	const linkedReview = waitForMessage(second, message => message.type === 'event' && message.event?.kind === 'review' && message.event.taskId === taskId);
	first.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'review', detail: `Review for ${task.title}: ready`, taskId }));
	assert.match((await linkedReview).event.detail, /Review for Run the integration checks/);
	const patch = 'diff --git a/src/main.ts b/src/main.ts\n@@ -1 +1 @@\n-old\n+new';
	const firstDiff = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'diff' && message.event.file === 'src/main.ts');
	const secondDiff = waitForMessage(second, message => message.type === 'event' && message.event?.kind === 'diff' && message.event.file === 'src/main.ts');
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'diff', detail: 'Reviewed Git patch', file: 'src/main.ts', patch }));
	const diffEvent = (await firstDiff).event;
	assert.equal((await secondDiff).event.patch, patch);
	assert.deepEqual({ actor: diffEvent.participantId, path: diffEvent.file, patch: diffEvent.patch, timestamp: Date.parse(diffEvent.at) > 0 }, { actor: secondId, path: 'src/main.ts', patch, timestamp: true });
	const anchor = { startLine: 8, endLine: 9 };
	const firstComment = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'comment' && message.event.file === 'src/main.ts');
	const secondComment = waitForMessage(second, message => message.type === 'event' && message.event?.kind === 'comment' && message.event.file === 'src/main.ts');
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'comment', detail: 'Please check this boundary case.', file: 'src/main.ts', anchor }));
	const commentEvent = (await firstComment).event;
	assert.deepEqual({ actor: commentEvent.participantId, detail: commentEvent.detail, path: commentEvent.file, anchor: commentEvent.anchor, includesSelectedText: Object.hasOwn(commentEvent, 'selectedText'), timestamp: Date.parse(commentEvent.at) > 0 }, { actor: secondId, detail: 'Please check this boundary case.', path: 'src/main.ts', anchor, includesSelectedText: false, timestamp: true });
	assert.deepEqual((await secondComment).event.anchor, anchor);
	const rejectedPrivateComment = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'comment' && message.event.detail === 'must not include code', 100);
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'comment', detail: 'must not include code', file: 'src/main.ts', anchor, selectedText: 'private source' }));
	await assert.rejects(rejectedPrivateComment, /Timed out waiting for room message/);
	const missingReviewTask = waitForMessage(first, message => message.type === 'notice' && message.detail.includes('linked review task'));
	first.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'review', detail: 'Unknown task review', taskId: randomUUID() }));
	await missingReviewTask;
	const deniedAgentLink = waitForMessage(first, message => message.type === 'notice' && message.detail.includes('assigned task owner'));
	first.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'agent', detail: 'Build started', taskId }));
	assert.match((await deniedAgentLink).detail, /Only the assigned task owner or host/);
	const linkedAgentEvent = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'agent' && message.event.taskId === taskId);
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'agent', detail: 'Build started', taskId }));
	assert.equal((await linkedAgentEvent).event.taskId, taskId);
	const rejectedPresence = waitForMessage(first, message => message.type === 'notice' && message.detail.includes('Agent presence must be linked'));
	first.send(JSON.stringify({ version: protocolVersion, type: 'agentPresence', presence: { id: randomUUID(), name: 'Agent X', role: 'agent', ownerId: firstId, status: 'working', taskId } }));
	await rejectedPresence;
	const activePresence = waitForMessage(first, message => message.type === 'count' && message.members.some(member => member.role === 'agent' && member.taskId === taskId));
	second.send(JSON.stringify({ version: protocolVersion, type: 'agentPresence', presence: { id: randomUUID(), name: 'Model Agent', role: 'agent', ownerId: secondId, status: 'working', taskId, prompt: 'private text must not be broadcast' } }));
	const presenceSnapshot = await activePresence;
	const agentSnapshot = presenceSnapshot.members.find(member => member.role === 'agent' && member.taskId === taskId);
	assert.equal(agentSnapshot.ownerId, secondId);
	assert.equal(Object.hasOwn(agentSnapshot, 'prompt'), false);
	const clearedPresence = waitForMessage(first, message => message.type === 'count' && !message.members.some(member => member.role === 'agent' && member.taskId === taskId));
	second.send(JSON.stringify({ version: protocolVersion, type: 'agentPresence', presence: null }));
	await clearedPresence;
	const deniedTaskChange = waitForMessage(first, message => message.type === 'notice' && message.detail.includes('current task owner'));
	first.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'task', detail: 'unauthorized reassignment', task: { ...task, ownerId: 'host-id', owner: 'Host' } }));
	assert.match((await deniedTaskChange).detail, /Only the host or current task owner/);
	const privateMessage = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'message' && message.event.detail === 'meet at the review table');
	const privateEcho = waitForMessage(second, message => message.type === 'event' && message.event?.kind === 'message' && message.event.detail === 'meet at the review table');
	const leakedToObserver = waitForMessage(observer, message => message.type === 'event' && message.event?.kind === 'message' && message.event.detail === 'meet at the review table', 100);
	second.send(JSON.stringify({ version: protocolVersion, type: 'event', kind: 'message', detail: 'meet at the review table', recipientId: firstId }));
	assert.equal((await privateMessage).event.recipientId, firstId);
	assert.equal((await privateEcho).event.senderId, secondId);
	await assert.rejects(leakedToObserver, /Timed out waiting for room message/);
	assert.equal(canDeliverMessageEvent({ kind: 'message', recipient: 'Ada', recipientId: firstId, senderId: secondId }, observerId), false);
	const observerClose = once(observer, 'close');
	observer.close();
	await observerClose;
	const observerReconnected = await connect(url, observerId, 'Sam');
	t.after(() => observerReconnected.terminate());
	const observerHistoryAfterMessage = await waitForMessage(observerReconnected, message => message.type === 'history');
	assert.equal(observerHistoryAfterMessage.events.some(event => event.kind === 'message'), false);
	assert.equal(observerHistoryAfterMessage.events.some(event => event.kind === 'diff' && event.file === 'src/main.ts'), true);
	assert.equal(observerHistoryAfterMessage.events.some(event => event.kind === 'comment' && event.anchor?.startLine === 8), true);

	const leave = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'leave' && message.event.participantId === secondId);
	const afterLeave = waitForMessage(first, message => message.type === 'count' && message.count === 3);
	second.close();
	await once(second, 'close');
	await leave;
	await afterLeave;

	const rejoin = waitForMessage(first, message => message.type === 'event' && message.event?.kind === 'join' && message.event.participantId === secondId);
	const rejoined = await connect(url, secondId, 'Lin');
	t.after(() => rejoined.terminate());
	const rejoinHistory = await waitForMessage(rejoined, message => message.type === 'history');
	assert.equal(rejoinHistory.members.filter(member => member.id === secondId).length, 1);
	assert.equal(rejoinHistory.tasks.find(item => item.id === taskId).ownerId, secondId);
	assert.equal(rejoinHistory.events.some(event => event.kind === 'message' && event.senderId === secondId), true);
	await rejoin;
	assert.equal(room.peers.size, 3);
});

test('two clients exchange throttled ephemeral editor locations and stop sharing cleanly', async t => {
	const { server, url, room } = await startServer(t, 'room-secret');
	const sharerId = randomUUID();
	const followerId = randomUUID();
	const sharer = await connect(url, sharerId, 'Ada');
	const sharerHistory = await waitForMessage(sharer, message => message.type === 'history');
	const follower = await connect(url, followerId, 'Lin');
	t.after(() => { sharer.terminate(); follower.terminate(); });
	await waitForMessage(follower, message => message.type === 'history');
	const received = waitForMessage(follower, message => message.type === 'editorLocation' && message.active === true);
	sharer.send(JSON.stringify({ version: protocolVersion, type: 'editorLocation', active: true, file: 'src/main.ts', line: 7 }));
	const locationMessage = await received;
	assert.deepEqual(locationMessage.location, { participantId: sharerId, name: 'Ada', role: 'human', file: 'src/main.ts', line: 7, updatedAt: locationMessage.location.updatedAt });
	assert.equal(Object.hasOwn(locationMessage.location, 'selectedText'), false);
	const ignoredInvalid = waitForMessage(follower, message => message.type === 'editorLocation' && message.location?.line === 8, 120);
	sharer.send(JSON.stringify({ version: protocolVersion, type: 'editorLocation', active: true, file: '../secret.ts', line: 8 }));
	await assert.rejects(ignoredInvalid, /Timed out waiting/);
	const stopped = waitForMessage(follower, message => message.type === 'editorLocation' && message.active === false && message.participantId === sharerId);
	sharer.send(JSON.stringify({ version: protocolVersion, type: 'editorLocation', active: false }));
	await stopped;
	assert.equal(room.events.some(event => event.type === 'editorLocation' || event.kind === 'editorLocation'), false);
	assert.equal(sharerHistory.type, 'history');
});

test('room transport rejects invalid invites and unsupported protocol versions', async t => {
	const { url } = await startServer(t, 'room-secret');
	const invalid = new WebSocket(url);
	trackSocket(invalid);
	const invalidClose = once(invalid, 'close');
	await once(invalid, 'open');
	invalid.send(JSON.stringify({ version: protocolVersion, type: 'hello', secret: 'wrong', participantId: randomUUID(), name: 'Mallory', role: 'human' }));
	const [invalidCode] = await invalidClose;
	assert.equal(invalidCode, 1008);

	const outdated = new WebSocket(url);
	trackSocket(outdated);
	const outdatedClose = once(outdated, 'close');
	await once(outdated, 'open');
	outdated.send(JSON.stringify({ version: protocolVersion - 1, type: 'hello' }));
	const [outdatedCode] = await outdatedClose;
	assert.equal(outdatedCode, 1002);
});

test('revoking an invite disconnects its active participant, records the action, and denies reconnect', async t => {
	const { url, room } = await startServer(t, 'room-secret');
	const hostObserver = await connect(url, randomUUID(), 'Host observer');
	t.after(() => hostObserver.terminate());
	await waitForMessage(hostObserver, message => message.type === 'history');
	const inviteToken = 'secondary-invite-token';
	room.addInvite(inviteToken);
	const guestId = randomUUID();
	const guest = await connect(url, guestId, 'Guest', inviteToken);
	t.after(() => guest.terminate());
	await waitForMessage(guest, message => message.type === 'history');
	const guestClosed = once(guest, 'close');
	const revokeEvent = waitForMessage(hostObserver, message => message.type === 'event' && message.event?.kind === 'revoke');
	assert.equal(room.revoke(inviteToken).peers.length, 1);
	const [closeCode] = await guestClosed;
	assert.equal(closeCode, 4003);
	const audit = (await revokeEvent).event;
	assert.equal(audit.participantId, 'host-id');
	assert.match(audit.detail, /disconnected 1 participant/);
	assert.equal(JSON.stringify(audit).includes(inviteToken), false);
	const rejected = new WebSocket(url);
	trackSocket(rejected);
	const rejectedClose = once(rejected, 'close');
	await once(rejected, 'open');
	rejected.send(JSON.stringify({ version: protocolVersion, type: 'hello', secret: inviteToken, participantId: guestId, name: 'Guest', role: 'human' }));
	const [rejectedCode] = await rejectedClose;
	assert.equal(rejectedCode, 1008);
});
