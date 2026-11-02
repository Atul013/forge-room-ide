const test = require('node:test');
const assert = require('node:assert/strict');
const { formatRoomTasksForModel } = require('../dist/roomTaskContext');

test('room task context includes bounded task metadata and excludes unrelated fields', () => {
	const result = formatRoomTasksForModel([{
		id: 'task-1', title: 'Implement context', owner: 'Ari', status: 'inProgress', priority: 'high', estimateMinutes: 90,
		dependencies: ['task-0'], acceptanceCriteria: ['Show exact transfer'], dueAt: '2026-10-22T12:00:00Z',
		notes: 'must not leak', chat: 'must not leak'
	}]);
	assert.match(result, /Implement context/);
	assert.match(result, /priority: high/);
	assert.match(result, /estimate: 90 minutes/);
	assert.match(result, /depends on: task-0/);
	assert.match(result, /Show exact transfer/);
	assert.doesNotMatch(result, /must not leak/);
});

test('room task context caps task count and character output', () => {
	const tasks = Array.from({ length: 55 }, (_, index) => ({ id: `t${index}`, title: `Task ${index}`, owner: 'owner', status: 'open' }));
	assert.match(formatRoomTasksForModel(tasks), /additional tasks omitted/);
	assert.equal(formatRoomTasksForModel(tasks, 50, 180).length, 180);
	assert.match(formatRoomTasksForModel(tasks, 50, 180), /truncated/);
});
