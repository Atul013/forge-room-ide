const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

function renderRoom(snapshot) {
	const app = { innerHTML: '', addEventListener() {} };
	const window = { roomSnapshot: snapshot, addEventListener() {} };
	const document = { getElementById: () => app };
	const source = fs.readFileSync(path.join(__dirname, '..', 'media', 'room.js'), 'utf8');
	vm.runInNewContext(source, { window, document, setInterval() {} });
	return app.innerHTML;
}

test('member cards show tasks by participant ID and keep duplicate names separate', () => {
	const html = renderRoom({
		participantId: 'sam-one',
		members: [
			{ id: 'sam-one', isSelf: true, name: 'Sam', role: 'human' },
			{ id: 'sam-two', name: 'Sam', role: 'agent' },
			{ id: 'rae', name: 'Rae', role: 'human' }
		],
		tasks: [
			{ id: 'a', ownerId: 'sam-one', owner: 'Sam', title: 'Socket tests', status: 'inProgress', updatedAt: new Date().toISOString() },
			{ id: 'b', ownerId: 'sam-two', owner: 'Sam', title: 'Docs pass', status: 'open', updatedAt: new Date().toISOString() },
			{ id: 'legacy', owner: 'Sam', title: 'Ambiguous legacy task', status: 'open', updatedAt: new Date().toISOString() },
			{ id: 'c', ownerId: 'rae', owner: 'Rae', title: '<write tests>', status: 'done', updatedAt: new Date().toISOString() }
		],
		events: [{ id: 'activity', kind: 'agent', name: 'Sam', role: 'human', detail: 'Build started for room task: Socket tests.', at: new Date().toISOString() }], notes: [], connected: true
	});
	assert.match(html, /Working on Socket tests · In motion/);
	assert.match(html, /person-name">Sam <span class="person-role">you<\/span>/);
	assert.match(html, /Working on Docs pass · Open/);
	assert.doesNotMatch(html, /Working on Ambiguous legacy task/);
	assert.match(html, /person-role">human/);
	assert.doesNotMatch(html, /person-role">human ·/);
	assert.doesNotMatch(html, /Working on &lt;write tests&gt;/);
	assert.match(html, /Sam worked with Forge Room Agent: Build started for room task: Socket tests\./);
});

test('task cards display live deadline and overdue states', () => {
	const now = Date.now();
	const html = renderRoom({
		members: [], events: [], notes: [], participantId: 'self',
		tasks: [
			{ id: 'future', title: 'Upcoming', owner: 'Ari', status: 'open', updatedAt: new Date(now).toISOString(), dueAt: new Date(now + 2 * 60 * 60_000).toISOString() },
			{ id: 'past', title: 'Late', owner: 'Bo', status: 'open', updatedAt: new Date(now).toISOString(), dueAt: new Date(now - 60 * 60_000).toISOString() }
		]
	});
	assert.match(html, /Due in 2h/);
	assert.match(html, /data-overdue="true"[^>]*>Overdue 1h/);
	assert.ok(html.indexOf('data-id="past"') < html.indexOf('data-id="future"'));
});

test('hackathon countdown appears to everyone but controls are host only', () => {
	const deadline = new Date(Date.now() + 3 * 60 * 60_000).toISOString();
	const host = renderRoom({ members: [], events: [], notes: [], tasks: [], hosting: true, sessionDeadline: deadline });
	assert.match(host, /Submission deadline/);
	assert.match(host, /3h 0m remaining/);
	assert.match(host, /data-action="setCountdown"/);
	const participant = renderRoom({ members: [], events: [], notes: [], tasks: [], connected: true, hosting: false, sessionDeadline: deadline });
	assert.match(participant, /Hackathon countdown/);
	assert.doesNotMatch(participant, /data-action="setCountdown"/);
});

test('paused hackathon countdown displays frozen time and host-only resume control', () => {
	const remaining = 3 * 60 * 60_000;
	const host = renderRoom({ members: [], events: [], notes: [], tasks: [], hosting: true, sessionPausedRemainingMs: remaining });
	assert.match(host, /Countdown paused/);
	assert.match(host, /Paused · 3h 0m remaining/);
	assert.match(host, /Time is frozen until the host resumes the countdown\./);
	assert.match(host, /data-action="toggleCountdown">Resume countdown/);
	const participant = renderRoom({ members: [], events: [], notes: [], tasks: [], connected: true, hosting: false, sessionPausedRemainingMs: remaining });
	assert.match(participant, /Hackathon countdown/);
	assert.doesNotMatch(participant, /data-action="toggleCountdown"/);
});

test('live Git diff sharing shows a stop control and explains snapshot retention', () => {
	const html = renderRoom({ members: [], events: [], notes: [], tasks: [], hosting: true, liveDiffSharing: true, liveDiffFile: 'src/example.ts' });
	assert.match(html, /Live Git diff sharing/);
	assert.match(html, /New saved diffs for src\/example\.ts are shared with the room/);
	assert.match(html, /Earlier snapshots remain in room history/);
	assert.match(html, /data-action="stopLiveDiff">Stop sharing/);
});
