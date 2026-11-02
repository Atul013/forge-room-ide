const test = require('node:test');
const assert = require('node:assert/strict');
const { formatWorkspaceTestTasks, matchingWorkspaceTestTasks } = require('../dist/workspaceTasks');

test('formats only registered test task labels and providers', () => {
	const result = formatWorkspaceTestTasks([
		{ name: 'unit tests', source: 'npm' },
		{ name: 'integration tests', source: 'Forge Provider' }
	]);
	assert.equal(result, 'Registered test tasks (2):\n- unit tests (provider: npm)\n- integration tests (provider: Forge Provider)');
});

test('sanitizes labels and bounds count and output', () => {
	const result = formatWorkspaceTestTasks([
		{ name: 'bad\nname\u0000', source: 'provider' },
		{ name: 'omitted', source: 'other' }
	], 1);
	assert.match(result, /bad name/);
	assert.match(result, /additional tasks omitted/);
	assert.equal(result.includes('\u0000'), false);
	assert.ok(formatWorkspaceTestTasks([{ name: 'x'.repeat(400), source: 'y' }], 100, 50).length <= 50);
});

test('handles empty task lists', () => {
	assert.equal(formatWorkspaceTestTasks([]), 'No registered test tasks were found in the selected workspace.');
});

test('selects tasks by exact sanitized name and provider, preserving duplicate matches for user selection', () => {
	const tasks = [
		{ name: 'unit tests', source: 'npm', id: 1 },
		{ name: 'unit tests', source: 'npm', id: 2 },
		{ name: 'unit tests', source: 'other', id: 3 }
	];
	assert.deepEqual(matchingWorkspaceTestTasks(tasks, 'unit tests', 'npm').map(task => task.id), [1, 2]);
	assert.deepEqual(matchingWorkspaceTestTasks(tasks, 'unit tests', 'missing'), []);
});
