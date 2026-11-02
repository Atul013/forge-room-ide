const test = require('node:test');
const assert = require('node:assert/strict');
const { isValidCompletedCriteria, taskCompletionDenial } = require('../dist/taskCompletionPolicy');

const criteria = ['API contract is documented', 'Tests pass'];
const openTask = { status: 'inProgress', acceptanceCriteria: criteria };

test('completion requires an owner confirmation for every acceptance criterion', () => {
	assert.match(taskCompletionDenial(openTask, { ...openTask, status: 'done' }), /2 remaining/);
	assert.match(taskCompletionDenial(openTask, { ...openTask, status: 'done', completedCriteria: [criteria[0]] }), /1 remaining/);
	assert.equal(taskCompletionDenial(openTask, { ...openTask, status: 'done', completedCriteria: criteria }), undefined);
	assert.equal(taskCompletionDenial({ ...openTask, acceptanceCriteria: ['same', 'same'] }, { status: 'done', acceptanceCriteria: ['same', 'same'], completedCriteria: ['same'] }), undefined);
});

test('confirmation values must be unique and belong to the task criteria', () => {
	assert.equal(isValidCompletedCriteria([criteria[0], criteria[0]], criteria), false);
	assert.equal(isValidCompletedCriteria(['another check'], criteria), false);
	assert.equal(isValidCompletedCriteria(criteria, criteria), true);
});

test('completed criteria cannot be edited until a completed task is reopened', () => {
	const done = { status: 'done', acceptanceCriteria: criteria, completedCriteria: criteria };
	assert.match(taskCompletionDenial(done, { ...done, acceptanceCriteria: [criteria[0]] }), /Reopen/);
	assert.equal(taskCompletionDenial(done, { ...done, status: 'open', completedCriteria: undefined }), undefined);
});

test('reopened tasks must clear old confirmations', () => {
	assert.match(taskCompletionDenial(openTask, { ...openTask, status: 'open', completedCriteria: [criteria[0]] }), /Clear acceptance/);
});
