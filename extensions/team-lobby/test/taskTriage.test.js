const test = require('node:test');
const assert = require('node:assert/strict');
const { triageRoomTasks } = require('../dist/taskTriage');

const now = Date.parse('2026-10-25T12:00:00.000Z');
const task = (id, extra = {}) => ({ id, title: id, owner: 'Sam', status: 'open', ...extra });

test('triage ranks runnable overdue and soon-due work before blocked tasks', () => {
	const result = triageRoomTasks([
		task('blocked', { dependencies: ['unfinished'], dueAt: '2026-10-25T12:10:00.000Z' }),
		task('unfinished', { status: 'inProgress' }),
		task('soon', { dueAt: '2026-10-25T13:00:00.000Z' }),
		task('overdue', { dueAt: '2026-10-25T11:30:00.000Z' }),
		task('done', { status: 'done', dueAt: '2026-10-25T11:00:00.000Z' })
	], now);
	assert.deepEqual(result.items.map(item => item.task.id), ['overdue', 'soon', 'unfinished', 'blocked']);
	assert.equal(result.openCount, 4);
	assert.equal(result.blockedCount, 1);
	assert.equal(result.overdueCount, 1);
	assert.equal(result.dueSoonCount, 2);
	assert.match(result.items[0].recommendation, /Overdue by 30m/);
	assert.match(result.items[3].recommendation, /Blocked by unfinished/);
});

test('triage counts unique transitive active dependents and omits completed descendants', () => {
	const result = triageRoomTasks([
		task('root'),
		task('middle', { dependencies: ['root'] }),
		task('leaf', { dependencies: ['middle', 'root'] }),
		task('done-leaf', { status: 'done', dependencies: ['middle'] })
	], now);
	assert.equal(result.items.find(item => item.task.id === 'root').downstreamCount, 2);
	assert.equal(result.items.find(item => item.task.id === 'middle').downstreamCount, 1);
	assert.equal(result.items.some(item => item.task.id === 'done-leaf'), false);
	assert.match(result.items.find(item => item.task.id === 'root').recommendation, /Unblocks 2 active tasks/);
});

test('triage tolerates invalid and missing deadline values and remains deterministic', () => {
	const result = triageRoomTasks([
		task('zeta', { dueAt: 'invalid' }),
		task('beta'),
		task('alpha')
	], now);
	assert.deepEqual(result.items.map(item => item.task.id), ['alpha', 'beta', 'zeta']);
	assert.equal(result.items.every(item => item.dueInMs === undefined), true);
	assert.equal(result.items[0].recommendation, 'Ready to start');
});

test('triage uses priority for equally urgent work and labels high priority recommendations', () => {
	const result = triageRoomTasks([
		task('normal'),
		task('critical', { priority: 'critical' }),
		task('high', { priority: 'high' }),
		task('low', { priority: 'low' })
	], now);
	assert.deepEqual(result.items.map(item => item.task.id), ['critical', 'high', 'normal', 'low']);
	assert.match(result.items[0].recommendation, /Critical priority/);
	assert.match(result.items[1].recommendation, /High priority/);
});

test('triage identifies the longest estimated unfinished dependency path', () => {
	const result = triageRoomTasks([
		task('api', { estimateMinutes: 60 }),
		task('ui', { estimateMinutes: 30, dependencies: ['api'] }),
		task('docs', { estimateMinutes: 20 })
	], now);
	assert.equal(result.criticalPathComplete, true);
	assert.equal(result.criticalPathMinutes, 90);
	assert.deepEqual(result.criticalPathTaskIds, ['api', 'ui']);
	assert.deepEqual(result.items.filter(item => item.criticalPath).map(item => item.task.id), ['api', 'ui']);
	assert.match(result.items.find(item => item.task.id === 'api').recommendation, /Critical path/);
});
