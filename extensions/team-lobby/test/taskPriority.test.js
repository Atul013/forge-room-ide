const test = require('node:test');
const assert = require('node:assert/strict');
const { isTaskPriority, taskPriorityLabel, taskPriorityRank } = require('../dist/taskPriority');

test('task priority is restricted to the supported set and old tasks default to normal', () => {
	assert.equal(isTaskPriority('critical'), true);
	assert.equal(isTaskPriority('urgent'), false);
	assert.equal(taskPriorityLabel(undefined), 'Normal');
	assert.equal(taskPriorityRank(undefined), taskPriorityRank('normal'));
});

test('higher priority receives a higher ordering score', () => {
	assert.ok(taskPriorityRank('critical') > taskPriorityRank('high'));
	assert.ok(taskPriorityRank('high') > taskPriorityRank('normal'));
	assert.ok(taskPriorityRank('normal') > taskPriorityRank('low'));
});
