const test = require('node:test');
const assert = require('node:assert/strict');
const { ensureAgentSessionId } = require('../dist/agentSessionIdentity');

test('room Build continuations preserve a valid participant ID', () => {
	const existing = 'ab7a6070-64ae-4c2d-aaf4-7b67112d0d9f';
	assert.equal(ensureAgentSessionId(existing, () => 'unexpected'), existing);
});

test('legacy Build sessions receive an ID once and reuse it on later continuations', () => {
	let agentId;
	let created = 0;
	const createId = () => { created++; return 'ab7a6070-64ae-4c2d-aaf4-7b67112d0d9f'; };
	agentId = ensureAgentSessionId(agentId, createId);
	agentId = ensureAgentSessionId(agentId, createId);
	assert.equal(agentId, 'ab7a6070-64ae-4c2d-aaf4-7b67112d0d9f');
	assert.equal(created, 1);
});

test('invalid persisted IDs are replaced with a valid generated ID', () => {
	const generated = 'ab7a6070-64ae-4c2d-aaf4-7b67112d0d9f';
	assert.equal(ensureAgentSessionId('not-an-id', () => generated), generated);
});
