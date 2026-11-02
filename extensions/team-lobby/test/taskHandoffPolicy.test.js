const test = require('node:test');
const assert = require('node:assert/strict');
const { taskHandoffDenial } = require('../dist/taskHandoffPolicy');

const task = { id: 'task-1', status: 'inProgress', ownerId: 'owner' };
const online = new Set(['owner', 'target', 'host']);

test('only the assigned owner can hand off an unfinished task to another connected participant', () => {
	assert.equal(taskHandoffDenial('owner', task, 'target', online), undefined);
	assert.match(taskHandoffDenial('host', task, 'target', online), /assigned task owner/);
	assert.equal(taskHandoffDenial('host', task, 'target', online, true), undefined);
	assert.match(taskHandoffDenial('host', task, 'offline', online, true), /connected/);
	assert.match(taskHandoffDenial('owner', task, 'offline', online), /connected/);
	assert.match(taskHandoffDenial('owner', task, 'owner', online), /other than|different/);
	assert.match(taskHandoffDenial('host', task, 'owner', online, true), /current task owner/);
});

test('completed and missing tasks cannot be handed off', () => {
	assert.match(taskHandoffDenial('owner', { ...task, status: 'done' }, 'target', online), /Completed tasks/);
	assert.match(taskHandoffDenial('owner', undefined, 'target', online), /no longer exists/);
});
