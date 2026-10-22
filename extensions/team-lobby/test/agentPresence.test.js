const test = require('node:test');
const assert = require('node:assert/strict');
const { isAgentPresence } = require('../dist/agentPresence');

const ownerId = '7b2e9c3a-6f1d-47a1-9d5d-078648591e24';
const base = { id: 'b648857a-2be8-49d6-8d12-0f16a86d7b68', name: 'Forge Room GPT', role: 'agent', ownerId, status: 'working', taskId: 'task-1' };

test('validates a bounded agent presence associated with its human owner and task', () => {
	assert.equal(isAgentPresence(base, ownerId), true);
	assert.equal(isAgentPresence({ ...base, status: 'review' }, ownerId), true);
	assert.equal(isAgentPresence(base, 'f129c8d0-8e24-4c22-a049-48c327ae0ef7'), false);
	assert.equal(isAgentPresence({ ...base, name: 'x'.repeat(41) }, ownerId), false);
	assert.equal(isAgentPresence({ ...base, taskId: 'bad\npath' }, ownerId), false);
	assert.equal(isAgentPresence({ ...base, shell: true }, ownerId), false);
});
