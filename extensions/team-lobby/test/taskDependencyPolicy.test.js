const test = require('node:test');
const assert = require('node:assert/strict');
const { taskDependencyDenial } = require('../dist/taskDependencyPolicy');

const base = [
	{ id: 'api', title: 'Define API', status: 'inProgress' },
	{ id: 'ui', title: 'Build UI', status: 'open', dependencies: ['api'] },
	{ id: 'tests', title: 'Add tests', status: 'open', dependencies: ['ui'] }
];

test('cannot start or complete a task while a direct dependency is unfinished', () => {
	assert.match(taskDependencyDenial(base[1], { ...base[1], status: 'inProgress' }, base), /Define API/);
	assert.match(taskDependencyDenial(base[1], { ...base[1], status: 'done' }, base), /Define API/);
	assert.equal(taskDependencyDenial(base[1], { ...base[1], status: 'done' }, [{ ...base[0], status: 'done' }, base[1], base[2]]), undefined);
});

test('cannot reopen a prerequisite when a transitive dependent has started', () => {
	const tasks = [{ ...base[0], status: 'done' }, base[1], { ...base[2], status: 'inProgress' }];
	assert.match(taskDependencyDenial(tasks[0], { ...tasks[0], status: 'open' }, tasks), /Add tests/);
	assert.equal(taskDependencyDenial(tasks[0], { ...tasks[0], status: 'open' }, [{ ...tasks[0], status: 'done' }, base[1], base[2]]), undefined);
});

test('a newly added unfinished dependency blocks an already active task', () => {
	const task = { id: 'ui', title: 'Build UI', status: 'inProgress', dependencies: ['api'] };
	assert.match(taskDependencyDenial(task, task, [task, ...base]), /Define API/);
});
