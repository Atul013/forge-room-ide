const test = require('node:test');
const assert = require('node:assert/strict');
const { deadlineAfterDuration, defaultHackathonDurationMs, isStoredTaskDeadline, parseFutureDeadlineInput, parseTaskDeadlineInput } = require('../dist/taskDeadline');
const { nextTaskDeadlineMilestone, taskDeadlineMilestones, taskDeadlineMilestonesToRecord, taskDeadlineMilestoneLabel, taskDeadlineTriggerAt, validTaskDeadlineAlertEntry, taskDeadlineAlertKey } = require('../dist/taskDeadlineAlerts');

test('normalizes explicit-timezone task deadlines to UTC', () => {
	assert.deepEqual(parseTaskDeadlineInput('2026-10-22T18:00+05:30'), { dueAt: '2026-10-22T12:30:00.000Z' });
	assert.deepEqual(parseTaskDeadlineInput('2026-10-22T12:30:00Z'), { dueAt: '2026-10-22T12:30:00.000Z' });
	assert.deepEqual(parseTaskDeadlineInput(''), {});
});

test('rejects timezone-free, invalid calendar, clock, and offset inputs', () => {
	for (const value of ['2026-10-22T12:30', '2026-02-30T12:30Z', '2026-10-22T25:30Z', '2026-10-22T12:30+25:00', '2026-10-22']) {
		assert.ok(parseTaskDeadlineInput(value).error, value);
	}
});

test('accepts only normalized UTC timestamps from room clients', () => {
	assert.equal(isStoredTaskDeadline('2026-10-22T12:30:00.000Z'), true);
	assert.equal(isStoredTaskDeadline('2026-10-22T12:30+05:30'), false);
	assert.equal(isStoredTaskDeadline('2026-02-30T12:30:00.000Z'), false);
});

test('hackathon countdown accepts future deadlines, rejects past ones, and allows clearing', () => {
	const now = Date.parse('2026-10-21T00:00:00.000Z');
	assert.deepEqual(parseFutureDeadlineInput('2026-10-22T12:00Z', now), { dueAt: '2026-10-22T12:00:00.000Z' });
	assert.match(parseFutureDeadlineInput('2026-10-20T12:00Z', now).error, /future/);
	assert.deepEqual(parseFutureDeadlineInput('', now), {});
});

test('24-hour hackathon preset uses a fixed duration from the host start time', () => {
	const now = Date.parse('2026-10-21T10:15:00.000Z');
	assert.equal(deadlineAfterDuration(defaultHackathonDurationMs, now), '2026-10-22T10:15:00.000Z');
	assert.equal(deadlineAfterDuration(0, now), undefined);
	assert.equal(deadlineAfterDuration(Number.MAX_SAFE_INTEGER, now), undefined);
});

const [fifteenMinutes, fiveMinutes, taskDeadline] = taskDeadlineMilestones;

test('task deadline reminders select 15 minutes, 5 minutes, and deadline once', () => {
	assert.equal(nextTaskDeadlineMilestone(30 * 60_000, new Set()), fifteenMinutes);
	assert.equal(nextTaskDeadlineMilestone(10 * 60_000, new Set([fifteenMinutes])), fiveMinutes);
	assert.equal(nextTaskDeadlineMilestone(-1, new Set([fifteenMinutes, fiveMinutes])), taskDeadline);
	assert.equal(nextTaskDeadlineMilestone(-1, new Set(taskDeadlineMilestones)), undefined);
});

test('missed task reminders coalesce and expired tasks do not replay old marks', () => {
	assert.deepEqual(taskDeadlineMilestonesToRecord(fifteenMinutes, 10 * 60_000), [fifteenMinutes]);
	assert.deepEqual(taskDeadlineMilestonesToRecord(fiveMinutes, 2 * 60_000), [fiveMinutes]);
	assert.deepEqual(taskDeadlineMilestonesToRecord(taskDeadline, -1), taskDeadlineMilestones);
	assert.equal(nextTaskDeadlineMilestone(-60_000, new Set()), taskDeadline);
});

test('task reminder records accept only hashed keys and supported marks', () => {
	assert.equal(validTaskDeadlineAlertEntry({ key: 'a'.repeat(64), milestones: [fifteenMinutes, taskDeadline] }), true);
	assert.equal(validTaskDeadlineAlertEntry({ key: 'task-id', milestones: [fifteenMinutes] }), false);
	assert.equal(validTaskDeadlineAlertEntry({ key: 'a'.repeat(64), milestones: [123] }), false);
	assert.match(taskDeadlineAlertKey('room', 'task', '2026-10-25T10:00:00.000Z'), /^[0-9a-f]{64}$/);
	assert.equal(taskDeadlineAlertKey('room', 'task', 'invalid'), undefined);
	assert.equal(taskDeadlineMilestoneLabel(taskDeadline), 'deadline reached');
	assert.equal(taskDeadlineTriggerAt(100 * 60_000, fifteenMinutes, 0), 85 * 60_000);
	assert.equal(taskDeadlineTriggerAt(10 * 60_000, fifteenMinutes, 0), 0);
	assert.equal(taskDeadlineTriggerAt(Number.NaN, fifteenMinutes, 0), undefined);
});
