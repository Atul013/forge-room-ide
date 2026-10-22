const test = require('node:test');
const assert = require('node:assert/strict');
const { agentActivityDetail, validTaskLink } = require('../dist/agentActivity');

test('Build room activity reports task, phase, count, checkpoint, and explicitly opted-in paths', () => {
	assert.equal(agentActivityDetail('started', 'Room sync'), 'Build started for room task: Room sync. Prompt and selected context remain private.');
	assert.match(agentActivityDetail('proposal', 'Room sync', 2), /proposal affecting 2 files.*ready for review.*no changes are applied/);
	assert.match(agentActivityDetail('applied', 'Room sync', 1, '1234567890'), /applied changes to 1 file.*Checkpoint 12345678/);
	assert.doesNotMatch(agentActivityDetail('applied', 'Room sync', 1, '1234567890'), /Changed paths/);
	assert.match(agentActivityDetail('applied', 'Room sync', 2, '1234567890', ['src/a.ts', 'src/b.ts']), /Changed paths: src\/a\.ts, src\/b\.ts/);
	assert.match(agentActivityDetail('paused', 'Room sync'), /paused.*No proposed files were applied.*resumed by its owner/);
	assert.doesNotMatch(agentActivityDetail('applied', 'Room sync', 5, '1234567890', ['a', 'b', 'c', 'd', 'e']), /a, b, c, d, e/);
	assert.ok(agentActivityDetail('applied', 'Room sync', 4, '1234567890', ['a'.repeat(60), 'b'.repeat(60), 'c'.repeat(60), 'd'.repeat(60)]).length <= 500);
	assert.equal(agentActivityDetail('started', 'x'.repeat(800)).length, 500);
	for (const stage of ['canceled', 'noProposal', 'incomplete', 'failed']) {
		assert.match(agentActivityDetail(stage, 'Room sync'), /No files were applied|No proposed files were applied/);
	}
});

test('Build activity never includes model prompts, context, or response arguments', () => {
	const detail = agentActivityDetail('proposal', 'Room sync', 1);
	assert.doesNotMatch(detail, /prompt|secret|context content/i);
	assert.match(agentActivityDetail('started', 'Room sync'), /Prompt and selected context remain private/);
});

test('task references are bounded and allowed on agent, verification, and review events', () => {
	assert.equal(validTaskLink('agent', 'task-123'), true);
	assert.equal(validTaskLink('test', 'task-123'), true);
	assert.equal(validTaskLink('review', 'task-123'), true);
	assert.equal(validTaskLink('post', 'task-123'), false);
	assert.equal(validTaskLink('agent', 'x'.repeat(101)), false);
	assert.equal(validTaskLink('agent', 'task\nother'), false);
	assert.equal(validTaskLink('agent', undefined), true);
});
