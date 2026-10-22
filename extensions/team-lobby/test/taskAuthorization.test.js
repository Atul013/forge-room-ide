const test = require('node:test');
const assert = require('node:assert/strict');
const { taskWriteDenial } = require('../dist/taskAuthorization');

const host = 'host-id';
const alice = 'alice-id';
const bob = 'bob-id';
const connected = new Set([host, alice, bob]);
const task = { id: 'task-1', status: 'open', ownerId: alice };

test('participants can create open tasks assigned to connected members', () => {
	assert.equal(taskWriteDenial(bob, host, undefined, { id: 'task-2', status: 'open', ownerId: alice }, connected), undefined);
	assert.match(taskWriteDenial(bob, host, undefined, { id: 'task-2', status: 'inProgress', ownerId: alice }, connected), /start open/);
	assert.match(taskWriteDenial(bob, host, undefined, { id: 'task-2', status: 'open', ownerId: 'offline' }, connected), /connected/);
});

test('only the assigned owner or host can update an existing task', () => {
	assert.match(taskWriteDenial(bob, host, task, { ...task, status: 'done' }, connected), /host or current task owner/);
	assert.equal(taskWriteDenial(alice, host, task, { ...task, status: 'inProgress' }, connected), undefined);
	assert.equal(taskWriteDenial(host, host, task, { ...task, status: 'done' }, connected), undefined);
});

test('only the host can reassign or reopen a completed task', () => {
	assert.match(taskWriteDenial(alice, host, task, { ...task, ownerId: bob }, connected), /Only the host can reassign/);
	assert.equal(taskWriteDenial(host, host, task, { ...task, ownerId: bob }, connected), undefined);
	assert.match(taskWriteDenial(alice, host, { ...task, status: 'done' }, { ...task, status: 'open' }, connected), /reopen/);
});

test('old ownerless tasks are host-managed until ownership is assigned', () => {
	assert.match(taskWriteDenial(alice, host, { id: task.id, status: 'open' }, { ...task }, connected), /host or current task owner/);
	assert.equal(taskWriteDenial(host, host, { id: task.id, status: 'open' }, { ...task }, connected), undefined);
});
