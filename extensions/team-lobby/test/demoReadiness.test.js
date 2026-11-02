const test = require('node:test');
const assert = require('node:assert/strict');
const { buildDemoReadiness, formatDemoReadinessMarkdown } = require('../dist/demoReadiness');

const now = Date.parse('2026-10-25T12:00:00.000Z');

test('readiness reflects unfinished, blocked, overdue tasks and latest linked verification', () => {
	const items = buildDemoReadiness([
		{ id: 'a', title: 'API', owner: 'Kim', status: 'inProgress', estimateMinutes: 60, dueAt: '2026-10-25T11:00:00Z' },
		{ id: 'b', title: 'Demo', owner: 'Lee', status: 'open', dependencies: ['a'] }
	], [
		{ id: 'old', at: '2026-10-25T10:00:00Z', kind: 'test', detail: 'verification for task “API”: tests passed (exit 0).', taskId: 'a' },
		{ id: 'new', at: '2026-10-25T11:00:00Z', kind: 'test', detail: 'verification for task “API”: tests failed (exit 1).', taskId: 'a' }
	], now);
	assert.equal(items[0].status, 'attention');
	assert.match(items[0].detail, /API \(Normal priority, inProgress, Kim, 1h estimate\)/);
	assert.equal(items[1].status, 'attention');
	assert.match(items[1].detail, /Demo/);
	assert.match(items[2].detail, /API/);
	assert.equal(items[4].status, 'attention');
	assert.match(items[4].detail, /failed/);
});

test('readiness leaves missing verification and external demo gates unknown', () => {
	const items = buildDemoReadiness([], [], now);
	assert.equal(items[0].status, 'unknown');
	assert.equal(items[4].status, 'unknown');
	assert.equal(items[5].status, 'unknown');
	assert.equal(items[6].status, 'unknown');
	assert.equal(items[7].status, 'unknown');
});

test('participant-reported verification never becomes a recorded passing check', () => {
	const manualPass = buildDemoReadiness([], [
		{ id: 'manual', at: '2026-10-25T11:00:00Z', kind: 'test', detail: 'manually reported verification for task “API”: tests passed (exit 0).' }
	], now);
	assert.equal(manualPass[4].status, 'unknown');
	assert.match(manualPass[4].detail, /not independently verified/);
	const actualThenManual = buildDemoReadiness([], [
		{ id: 'actual', at: '2026-10-25T10:00:00Z', kind: 'test', detail: 'verification for task “API”: tests passed (exit 0).' },
		{ id: 'manual-newer', at: '2026-10-25T11:00:00Z', kind: 'test', detail: 'manually reported verification for task “API”: tests failed (exit 1).' }
	], now);
	assert.equal(actualThenManual[4].status, 'unknown');
	assert.match(actualThenManual[4].detail, /newer participant-reported result is not verified/);
	const actualAfterManual = buildDemoReadiness([], [
		{ id: 'manual', at: '2026-10-25T10:00:00Z', kind: 'test', detail: 'manually reported verification for task “API”: tests failed (exit 1).' },
		{ id: 'actual-newer', at: '2026-10-25T11:00:00Z', kind: 'test', detail: 'verification for task “API”: tests passed (exit 0).' }
	], now);
	assert.equal(actualAfterManual[4].status, 'ready');
	assert.match(actualAfterManual[4].detail, /Participant-reported results are excluded/);
});

test('an older unlinked failure is superseded by a newer verification pass', () => {
	const items = buildDemoReadiness([], [
		{ id: 'failed', at: '2026-10-25T10:00:00Z', kind: 'test', detail: 'verification failed (exit 1).' },
		{ id: 'passed', at: '2026-10-25T11:00:00Z', kind: 'test', detail: 'verification passed (exit 0).' }
	], now);
	assert.equal(items[4].status, 'ready');
});

test('readiness reports the shared countdown and treats paused or passed deadlines as attention', () => {
	const absent = buildDemoReadiness([], [], now);
	assert.equal(absent[8].status, 'unknown');
	assert.match(absent[8].detail, /No shared room countdown/);
	const active = buildDemoReadiness([], [], now, { deadline: '2026-10-25T15:00:00.000Z' });
	assert.equal(active[8].status, 'ready');
	assert.match(active[8].detail, /3h remaining/);
	const paused = buildDemoReadiness([], [], now, { deadline: '2026-10-25T15:00:00.000Z', pausedRemainingMs: 30 * 60_000 });
	assert.equal(paused[8].status, 'attention');
	assert.match(paused[8].detail, /paused with 30m remaining/);
	const expired = buildDemoReadiness([], [], now, { deadline: '2026-10-25T11:00:00.000Z' });
	assert.equal(expired[8].status, 'attention');
	assert.match(expired[8].detail, /deadline has passed/);
});

test('readiness compares complete estimated critical path with active countdown', () => {
	const tasks = [
		{ id: 'a', title: 'API', owner: 'Kim', status: 'open', estimateMinutes: 90 },
		{ id: 'b', title: 'Demo', owner: 'Lee', status: 'open', estimateMinutes: 60, dependencies: ['a'] }
	];
	const enough = buildDemoReadiness(tasks, [], now, { deadline: '2026-10-25T16:00:00.000Z' });
	assert.equal(enough[9].status, 'ready');
	assert.match(enough[9].detail, /2h 30m.*4h remain/);
	const short = buildDemoReadiness(tasks, [], now, { deadline: '2026-10-25T13:00:00.000Z' });
	assert.equal(short[9].status, 'attention');
	assert.match(short[9].detail, /longer than the 1h remaining/);
	const paused = buildDemoReadiness(tasks, [], now, { pausedRemainingMs: 4 * 60 * 60_000 });
	assert.equal(paused[9].status, 'unknown');
	assert.match(paused[9].detail, /countdown is paused/);
});

test('readiness does not infer critical path feasibility from missing estimates or countdown', () => {
	const unknownEstimate = buildDemoReadiness([{ id: 'a', title: 'API', owner: 'Kim', status: 'open' }], [], now, { deadline: '2026-10-25T16:00:00.000Z' });
	assert.equal(unknownEstimate[9].status, 'unknown');
	const unknownCountdown = buildDemoReadiness([{ id: 'a', title: 'API', owner: 'Kim', status: 'open', estimateMinutes: 60 }], [], now);
	assert.equal(unknownCountdown[9].status, 'unknown');
});

test('readiness flags passed or near room milestones and labels future milestones', () => {
	const items = buildDemoReadiness([], [], now, { milestones: [
		{ id: 'passed', label: 'Code freeze', dueAt: '2026-10-25T11:00:00.000Z' },
		{ id: 'soon', label: 'Submit', dueAt: '2026-10-25T12:30:00.000Z' },
		{ id: 'later', label: 'Presentation', dueAt: '2026-10-25T16:00:00.000Z' }
	] });
	assert.equal(items[10].status, 'attention');
	assert.match(items[10].detail, /Code freeze \(passed\)/);
	assert.match(items[10].detail, /Submit \(due in 30m\)/);
	const unset = buildDemoReadiness([], [], now);
	assert.equal(unset[10].status, 'unknown');
});

test('readiness compares estimated remaining chain with milestone time and stays unknown without estimates', () => {
	const milestone = [{ id: 'm1', label: 'Code freeze', dueAt: '2026-10-25T14:00:00.000Z' }];
	const atRisk = buildDemoReadiness([{ id: 'a', title: 'API', owner: 'Kim', status: 'open', estimateMinutes: 180 }], [], now, { milestones: milestone });
	assert.equal(atRisk[10].status, 'attention');
	assert.match(atRisk[10].detail, /Code freeze \(estimated work exceeds available time\)/);
	const fits = buildDemoReadiness([{ id: 'a', title: 'API', owner: 'Kim', status: 'open', estimateMinutes: 60 }], [], now, { milestones: milestone });
	assert.equal(fits[10].status, 'ready');
	assert.match(fits[10].detail, /estimated work fits/);
	const unknown = buildDemoReadiness([{ id: 'a', title: 'API', owner: 'Kim', status: 'open' }], [], now, { milestones: milestone });
	assert.equal(unknown[10].status, 'unknown');
	assert.match(unknown[10].detail, /estimate unknown/);
});

test('milestone feasibility excludes completed work from the remaining chain', () => {
	const milestone = [{ id: 'm1', label: 'Code freeze', dueAt: '2026-10-25T14:00:00.000Z' }];
	const tasks = [
		{ id: 'a', title: 'API', owner: 'Kim', status: 'open', estimateMinutes: 60 },
		{ id: 'b', title: 'Demo', owner: 'Lee', status: 'open', estimateMinutes: 180, dependencies: ['a'] }
	];
	const fullyEstimated = buildDemoReadiness(tasks, [], now, { milestones: milestone });
	assert.equal(fullyEstimated[10].status, 'attention');
	const oneLeft = buildDemoReadiness([{ ...tasks[0], status: 'done' }, { ...tasks[1], estimateMinutes: 60, dependencies: [] }], [], now, { milestones: milestone });
	assert.equal(oneLeft[10].status, 'ready');
	assert.match(oneLeft[10].detail, /estimated work fits/);
	const missingRemaining = buildDemoReadiness([{ ...tasks[0], status: 'done' }, { ...tasks[1], estimateMinutes: undefined, dependencies: [] }], [], now, { milestones: milestone });
	assert.equal(missingRemaining[10].status, 'unknown');
	assert.match(missingRemaining[10].detail, /estimate unknown/);
});

test('markdown escapes control whitespace and labels unknown gates honestly', () => {
	const markdown = formatDemoReadinessMarkdown('Demo [Room](javascript:alert(1))\nRoom', '2026-10-25T12:00:00.000Z', [{ label: 'Preview', status: 'unknown', detail: 'Check manually.' }]);
	assert.match(markdown, /Demo \\\[Room\\\]/);
	assert.ok(markdown.includes(String.raw`\[Room\]\(javascript:alert\(1\)\)`));
	assert.match(markdown, /Unknown \/ manual check/);
	assert.match(markdown, /Check manually/);
});
